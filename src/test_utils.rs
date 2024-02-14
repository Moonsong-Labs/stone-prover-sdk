#![cfg(test)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cairo_vm::air_private_input::{AirPrivateInput, AirPrivateInputSerializable};
use rstest::fixture;
use tempfile::NamedTempFile;

use crate::json::read_json_from_file;
use crate::models::{Proof, ProverConfig, ProverParameters, PublicInput};

#[fixture]
pub fn prover_in_path() {
    // Add build dir to path for the duration of the test
    let path = std::env::var("PATH").unwrap_or_default();
    let build_dir = Path::new(env!("OUT_DIR"));
    // This will find the root of the target directory where the prover binaries
    // are put after compilation.
    let target_dir = build_dir.join("../../..").canonicalize().unwrap();

    std::env::set_var("PATH", format!("{}:{path}", target_dir.to_string_lossy()));
}

/// Reads and deserializes a JSON proof file.
pub fn read_proof_file<P: AsRef<Path>>(proof_file: P) -> Proof {
    let proof: Proof = read_json_from_file(proof_file).expect("Could not open proof file");
    proof
}

/// All the files forming a complete prover test case.
pub struct ProverTestCase {
    pub program_file: PathBuf,
    pub compiled_program_file: PathBuf,
    pub public_input_file: PathBuf,
    pub private_input_file: PathBuf,
    pub prover_config_file: PathBuf,
    pub prover_parameter_file: PathBuf,
    pub memory_file: PathBuf,
    pub trace_file: PathBuf,
    pub proof_file: PathBuf,
}

fn get_test_case_file_path(filename: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("dependencies")
        .join("cairo-programs")
        .join("cairo0")
        .join(filename)
}

pub fn load_test_case_file(filename: &str) -> String {
    let fixture_path = get_test_case_file_path(filename);
    std::fs::read_to_string(fixture_path).expect("Failed to read the fixture file")
}

#[fixture]
pub fn prover_test_case(#[default("fibonacci")] test_case: String) -> ProverTestCase {
    let program_file = get_test_case_file_path(&format!("{test_case}/{test_case}.cairo"));
    let compiled_program_file = get_test_case_file_path(&format!("{test_case}/fibonacci.json"));
    let public_input_file = get_test_case_file_path(&format!("{test_case}/air_public_input.json"));
    let private_input_file =
        get_test_case_file_path(&format!("{test_case}/air_private_input.json"));
    let prover_config_file =
        get_test_case_file_path(&format!("{test_case}/cpu_air_prover_config.json"));
    let prover_parameter_file =
        get_test_case_file_path(&format!("{test_case}/cpu_air_params.json"));
    let memory_file = get_test_case_file_path(&format!("{test_case}/memory.bin"));
    let trace_file = get_test_case_file_path(&format!("{test_case}/trace.bin"));
    let proof_file = get_test_case_file_path(&format!("{test_case}/proof.json"));

    ProverTestCase {
        program_file,
        compiled_program_file,
        public_input_file,
        private_input_file,
        prover_config_file,
        prover_parameter_file,
        memory_file,
        trace_file,
        proof_file,
    }
}

/// Test case files adapted to match the prover command line arguments.
pub struct ProverCliTestCase {
    pub public_input_file: PathBuf,
    pub private_input_file: NamedTempFile,
    pub prover_config_file: PathBuf,
    pub prover_parameter_file: PathBuf,
    pub proof: Proof,
}

#[fixture]
pub fn prover_cli_test_case(prover_test_case: ProverTestCase) -> ProverCliTestCase {
    // Generate the private input in a temporary file
    let private_input_file =
        NamedTempFile::new().expect("Creating temporary private input file failed");
    let private_input = AirPrivateInput(HashMap::new()).to_serializable(
        prover_test_case.trace_file.to_string_lossy().into_owned(),
        prover_test_case.memory_file.to_string_lossy().into_owned(),
    );

    serde_json::to_writer(&private_input_file, &private_input)
        .expect("Writing private input file failed");

    let proof = read_proof_file(&prover_test_case.proof_file);

    ProverCliTestCase {
        public_input_file: prover_test_case.public_input_file,
        private_input_file,
        prover_config_file: prover_test_case.prover_config_file,
        prover_parameter_file: prover_test_case.prover_parameter_file,
        proof,
    }
}

pub struct ParsedProverTestCase {
    pub compiled_program: Vec<u8>,
    pub public_input: PublicInput,
    pub private_input: AirPrivateInput,
    pub memory: Vec<u8>,
    pub trace: Vec<u8>,
    pub prover_config: ProverConfig,
    pub prover_parameters: ProverParameters,
    pub proof: Proof,
}

#[fixture]
pub fn parsed_prover_test_case(prover_test_case: ProverTestCase) -> ParsedProverTestCase {
    let compiled_program = std::fs::read(prover_test_case.compiled_program_file).unwrap();
    let public_input: PublicInput =
        read_json_from_file(prover_test_case.public_input_file).unwrap();
    let private_input: AirPrivateInputSerializable =
        read_json_from_file(prover_test_case.private_input_file).unwrap();
    let prover_config: ProverConfig =
        read_json_from_file(prover_test_case.prover_config_file).unwrap();
    let prover_parameters: ProverParameters =
        read_json_from_file(prover_test_case.prover_parameter_file).unwrap();
    let memory = std::fs::read(prover_test_case.memory_file).unwrap();
    let trace = std::fs::read(prover_test_case.trace_file).unwrap();

    let proof = read_proof_file(&prover_test_case.proof_file);

    ParsedProverTestCase {
        compiled_program,
        public_input,
        private_input: private_input.into(),
        memory,
        trace,
        prover_config,
        prover_parameters,
        proof,
    }
}
