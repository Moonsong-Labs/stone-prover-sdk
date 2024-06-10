use std::any::Any;
use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use cairo_bootloader::{
    BootloaderConfig, BootloaderInput, PackedOutput, SimpleBootloaderInput, Task, TaskSpec,
};
use cairo_vm::air_private_input::AirPrivateInput;
use cairo_vm::air_public_input::PublicInputError;
use cairo_vm::cairo_run::{
    write_encoded_memory, write_encoded_trace, CairoRunConfig, EncodeTraceError,
};
use cairo_vm::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
use cairo_vm::hint_processor::hint_processor_definition::HintProcessor;
use cairo_vm::types::errors::program_errors::ProgramError;
use cairo_vm::types::program::Program;
use cairo_vm::vm::errors::cairo_run_errors::CairoRunError;
use cairo_vm::vm::errors::trace_errors::TraceError;
use cairo_vm::vm::errors::vm_exception::VmException;
use cairo_vm::vm::runners::cairo_pie::CairoPie;
use cairo_vm::vm::runners::cairo_runner::CairoRunner;
use cairo_vm::vm::security::verify_secure_runner;
use cairo_vm::{any_box, Felt252};
use thiserror::Error;

use bincode::error::EncodeError;

use crate::models::{Layout, PublicInput};

// Copied from cairo_run.rs and adapted to support injecting the bootloader input.
// TODO: check if modifying CairoRunConfig to specify custom variables is accepted upstream.
pub fn cairo_run(
    program: &Program,
    cairo_run_config: &CairoRunConfig,
    hint_executor: &mut dyn HintProcessor,
    variables: HashMap<String, Box<dyn Any>>,
) -> Result<CairoRunner, CairoRunError> {
    let secure_run = cairo_run_config
        .secure_run
        .unwrap_or(!cairo_run_config.proof_mode);

    let allow_missing_builtins = cairo_run_config.allow_missing_builtins.unwrap_or(false);

    let mut cairo_runner = CairoRunner::new(
        program,
        cairo_run_config.layout,
        cairo_run_config.proof_mode,
        cairo_run_config.trace_enabled,
    )?;
    for (key, value) in variables {
        cairo_runner.exec_scopes.insert_box(&key, value);
    }

    let end = cairo_runner.initialize(allow_missing_builtins)?;
    // check step calculation

    cairo_runner
        .run_until_pc(end, hint_executor)
        .map_err(|err| VmException::from_vm_error(&cairo_runner, err))?;
    cairo_runner.end_run(cairo_run_config.disable_trace_padding, false, hint_executor)?;

    cairo_runner.read_return_values(allow_missing_builtins)?;
    if cairo_run_config.proof_mode {
        cairo_runner.finalize_segments()?;
    }
    if secure_run {
        verify_secure_runner(&cairo_runner, true, None)?;
    }
    cairo_runner.relocate(cairo_run_config.relocate_mem)?;

    Ok(cairo_runner)
}

/// Run a Cairo program in proof mode.
///
/// * `program_content`: Compiled program content.
pub fn run_in_proof_mode(
    program_content: &[u8],
    layout: Layout,
    allow_missing_builtins: Option<bool>,
) -> Result<CairoRunner, CairoRunError> {
    let proof_mode = true;
    let cairo_run_config = CairoRunConfig {
        entrypoint: "main",
        trace_enabled: true,
        relocate_mem: true,
        layout: layout.into(),
        proof_mode,
        secure_run: None,
        disable_trace_padding: false,
        allow_missing_builtins,
    };

    let mut hint_processor = BuiltinHintProcessor::new_empty();

    let runner =
        cairo_vm::cairo_run::cairo_run(program_content, &cairo_run_config, &mut hint_processor)?;
    Ok(runner)
}

#[derive(thiserror::Error, Debug)]
pub enum BootloaderTaskError {
    #[error("Failed to read program: {0}")]
    Program(#[from] ProgramError),

    #[error("Failed to read PIE: {0}")]
    Pie(#[from] io::Error),
}

pub fn make_bootloader_tasks(
    programs: &[Vec<u8>],
    pies: &[Vec<u8>],
) -> Result<Vec<TaskSpec>, BootloaderTaskError> {
    let program_tasks = programs.iter().map(|program_bytes| {
        let program = Program::from_bytes(program_bytes, Some("main"));
        program
            .map(|program| TaskSpec {
                task: Task::Program(program),
            })
            .map_err(BootloaderTaskError::Program)
    });

    let cairo_pie_tasks = pies.iter().map(|pie_bytes| {
        let pie = CairoPie::from_bytes(pie_bytes);
        pie.map(|pie| TaskSpec {
            task: Task::Pie(pie),
        })
        .map_err(BootloaderTaskError::Pie)
    });

    program_tasks.chain(cairo_pie_tasks).collect()
}

pub struct ExecutionArtifacts {
    pub public_input: PublicInput,
    pub private_input: AirPrivateInput,
    pub memory: Vec<u8>,
    pub trace: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum ExecutionError {
    #[error(transparent)]
    RunFailed(#[from] CairoRunError),
    #[error(transparent)]
    GeneratePublicInput(#[from] PublicInputError),
    #[error(transparent)]
    GenerateTrace(#[from] TraceError),
    #[error(transparent)]
    EncodeMemory(EncodeTraceError),
    #[error(transparent)]
    EncodeTrace(EncodeTraceError),
    #[error(transparent)]
    SerializePublicInput(#[from] serde_json::Error),
}

/// An in-memory writer for bincode encoding.
#[derive(Default)]
pub struct MemWriter {
    pub buf: Vec<u8>,
}

impl MemWriter {
    pub fn new() -> Self {
        Self::default()
    }
}

impl bincode::enc::write::Writer for MemWriter {
    fn write(&mut self, bytes: &[u8]) -> Result<(), EncodeError> {
        self.buf.extend_from_slice(bytes);
        Ok(())
    }
}

/// Extracts execution artifacts from the runner and VM (after execution).
///
/// * `cairo_runner` Cairo runner object.
pub fn extract_execution_artifacts(
    cairo_runner: CairoRunner,
) -> Result<ExecutionArtifacts, ExecutionError> {
    let memory = &cairo_runner.relocated_memory;
    let trace = cairo_runner
        .relocated_trace
        .as_ref()
        .ok_or(ExecutionError::GenerateTrace(TraceError::TraceNotEnabled))?;

    let mut memory_writer = MemWriter::new();
    write_encoded_memory(memory, &mut memory_writer).map_err(ExecutionError::EncodeMemory)?;
    let memory_raw = memory_writer.buf;

    let mut trace_writer = MemWriter::new();
    write_encoded_trace(trace, &mut trace_writer).map_err(ExecutionError::EncodeTrace)?;
    let trace_raw = trace_writer.buf;

    let cairo_vm_public_input = cairo_runner.get_air_public_input()?;
    let public_input = PublicInput::try_from(cairo_vm_public_input)?;

    let private_input = cairo_runner.get_air_private_input();

    Ok(ExecutionArtifacts {
        public_input,
        private_input,
        memory: memory_raw,
        trace: trace_raw,
    })
}

pub fn run_bootloader_in_proof_mode(
    bootloader: &Program,
    tasks: Vec<TaskSpec>,
    layout: Option<Layout>,
    allow_missing_builtins: Option<bool>,
    fact_topologies_path: Option<PathBuf>,
) -> Result<ExecutionArtifacts, ExecutionError> {
    let proof_mode = true;
    let layout = layout.unwrap_or(Layout::StarknetWithKeccak);

    let cairo_run_config = CairoRunConfig {
        entrypoint: "main",
        trace_enabled: true,
        relocate_mem: true,
        layout: layout.into(),
        proof_mode,
        secure_run: None,
        disable_trace_padding: false,
        allow_missing_builtins,
    };

    let n_tasks = tasks.len();

    let bootloader_input = BootloaderInput {
        simple_bootloader_input: SimpleBootloaderInput {
            fact_topologies_path,
            single_page: false,
            tasks,
        },
        bootloader_config: BootloaderConfig {
            simple_bootloader_program_hash: Felt252::from(0),
            supported_cairo_verifier_program_hashes: vec![],
        },
        packed_outputs: vec![PackedOutput::Plain(vec![]); n_tasks],
    };

    let mut hint_processor = BuiltinHintProcessor::new_empty();
    let variables = HashMap::<String, Box<dyn Any>>::from([
        ("bootloader_input".to_string(), any_box!(bootloader_input)),
        (
            "bootloader_program".to_string(),
            any_box!(bootloader.clone()),
        ),
    ]);

    let cairo_runner = cairo_run(
        bootloader,
        &cairo_run_config,
        &mut hint_processor,
        variables,
    )?;

    extract_execution_artifacts(cairo_runner)
}
