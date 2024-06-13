/// TODO: Move all this to cairo_bootloader repo?
use std::any::Any;
use std::collections::HashMap;
use std::path::PathBuf;

use cairo_bootloader::{
    insert_bootloader_input, BootloaderConfig, BootloaderHintProcessor, BootloaderInput, PackedOutput, SimpleBootloaderInput, TaskSpec
};
use cairo_vm::air_private_input::AirPrivateInput;
use cairo_vm::air_public_input::PublicInputError;
use cairo_vm::cairo_run::{
    cairo_run_program_with_initial_scope, write_encoded_memory, write_encoded_trace,
    CairoRunConfig, EncodeTraceError,
};
use cairo_vm::types::exec_scope::ExecutionScopes;
use cairo_vm::types::program::Program;
use cairo_vm::vm::errors::cairo_run_errors::CairoRunError;
use cairo_vm::vm::errors::trace_errors::TraceError;
use cairo_vm::vm::runners::cairo_runner::CairoRunner;
use cairo_vm::{any_box, Felt252};
use thiserror::Error;

use bincode::error::EncodeError;

use crate::models::{Layout, PublicInput};



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

    let mut hint_processor = BootloaderHintProcessor::new();

    let runner =
        cairo_vm::cairo_run::cairo_run(program_content, &cairo_run_config, &mut hint_processor)?;
    Ok(runner)
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

    let mut hint_processor = BootloaderHintProcessor::new();
    // let variables = HashMap::<String, Box<dyn Any>>::from([
    //     ("bootloader_input".to_string(), any_box!(bootloader_input)),
    //     (
    //         "bootloader_program".to_string(),
    //         any_box!(bootloader.clone()),
    //     ),
    // ]);
    // let scope = ExecutionScopes {
    //     data: vec![variables],
    // };

    let bootloader_identifiers = HashMap::from(
        [
            ("starkware.cairo.bootloaders.simple_bootloader.execute_task.execute_task.ret_pc_label".to_string(), 10usize),
            ("starkware.cairo.bootloaders.simple_bootloader.execute_task.execute_task.call_task".to_string(), 8usize)
        ]
    );
    let mut exec_scopes = ExecutionScopes::new();
    insert_bootloader_input(&mut exec_scopes, bootloader_input);
    exec_scopes.insert_value("bootloader_program_identifiers", bootloader_identifiers);
    exec_scopes.insert_value("bootloader_program", bootloader.clone());

    let cairo_runner = cairo_run_program_with_initial_scope(
        bootloader,
        &cairo_run_config,
        &mut hint_processor,
        exec_scopes,
    )?;

    extract_execution_artifacts(cairo_runner)
}


