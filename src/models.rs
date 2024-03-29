use cairo_vm::air_private_input::AirPrivateInputSerializable;
use stark_evm_adapter::annotation_parser::SplitProofs;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone)]
pub enum Verifier {
    Stone,
    L1,
}

impl FromStr for Verifier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let verifier = match s {
            "stone" => Self::Stone,
            "l1" => Self::L1,
            other => {
                return Err(format!("unknown verifier: {other}"));
            }
        };

        Ok(verifier)
    }
}

impl Display for Verifier {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Stone => "stone",
            Self::L1 => "l1",
        };
        write!(f, "{}", s)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CachedLdeConfig {
    pub store_full_lde: bool,
    pub use_fft_for_eval: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ProverConfig {
    pub cached_lde_config: CachedLdeConfig,
    pub constraint_polynomial_task_size: i32,
    pub n_out_of_memory_merkle_layers: i32,
    pub table_prover_n_tasks_per_segment: i32,
}

impl Default for ProverConfig {
    fn default() -> Self {
        Self {
            cached_lde_config: CachedLdeConfig {
                store_full_lde: false,
                use_fft_for_eval: false,
            },
            constraint_polynomial_task_size: 256,
            n_out_of_memory_merkle_layers: 1,
            table_prover_n_tasks_per_segment: 32,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct FriParameters {
    pub fri_step_list: Vec<u32>,
    pub last_layer_degree_bound: u32,
    pub n_queries: u32,
    pub proof_of_work_bits: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct StarkParameters {
    pub fri: FriParameters,
    pub log_n_cosets: i32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct ProverParameters {
    pub field: String,
    pub stark: StarkParameters,
    pub use_extension_field: bool,
}

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub enum Layout {
    #[serde(rename = "plain")]
    Plain,
    #[serde(rename = "small")]
    Small,
    #[serde(rename = "dex")]
    Dex,
    #[serde(rename = "recursive")]
    Recursive,
    #[serde(rename = "starknet")]
    Starknet,
    #[serde(rename = "recursive_large_output")]
    RecursiveLargeOutput,
    #[serde(rename = "all_cairo")]
    AllCairo,
    #[serde(rename = "all_solidity")]
    AllSolidity,
    #[serde(rename = "starknet_with_keccak")]
    StarknetWithKeccak,
}

impl FromStr for Layout {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_json::from_value::<Layout>(Value::String(s.to_string())).map_err(|e| e.to_string())
    }
}

impl Display for Layout {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = serde_json::to_value(self).map_err(|_| std::fmt::Error)?;
        let layout_str = value.as_str().expect("This is guaranteed to be a string");
        // serde_json adds
        write!(f, "{}", layout_str)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct MemorySegmentAddresses {
    pub begin_addr: u32,
    pub stop_ptr: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PublicMemoryEntry {
    pub address: u32,
    pub value: String,
    pub page: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct PublicInput {
    pub layout: Layout,
    pub rc_min: u32,
    pub rc_max: u32,
    pub n_steps: u32,
    pub memory_segments: HashMap<String, MemorySegmentAddresses>,
    pub public_memory: Vec<PublicMemoryEntry>,
    pub dynamic_params: Option<HashMap<String, u32>>,
}

// TODO: implement Deserialize in cairo-vm types.
impl<'a> TryFrom<cairo_vm::air_public_input::PublicInput<'a>> for PublicInput {
    type Error = serde_json::Error;

    /// Converts a Cairo VM `PublicInput` object into our format.
    ///
    /// Cairo VM provides an opaque public input struct that does not expose any of its members
    /// and only implements `Serialize`. Our only solution for now is to serialize this struct
    /// and deserialize it into our own format.
    fn try_from(value: cairo_vm::air_public_input::PublicInput<'a>) -> Result<Self, Self::Error> {
        // Cairo VM PublicInput does not expose members, so we are stuck with this poor
        // excuse of a conversion function for now.
        let public_input_str = serde_json::to_string(&value)?;
        serde_json::from_str::<Self>(&public_input_str)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProofVersion {
    commit_hash: String,
    proof_hash: String,
    statement_name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Proof {
    pub private_input: AirPrivateInputSerializable,
    pub proof_hex: String,
    pub proof_parameters: ProverParameters,
    pub prover_config: ProverConfig,
    pub public_input: PublicInput,
    pub split_proofs: Option<SplitProofs>,
}

#[derive(Debug)]
pub struct ProverWorkingDirectory {
    pub dir: tempfile::TempDir,
    pub public_input_file: PathBuf,
    pub private_input_file: PathBuf,
    pub _memory_file: PathBuf,
    pub _trace_file: PathBuf,
    pub prover_config_file: PathBuf,
    pub prover_parameter_file: PathBuf,
    pub proof_file: PathBuf,
    pub annotations_file: Option<PathBuf>,
    pub extra_annotations_file: Option<PathBuf>,
}

/// A struct representing the annotations artifacts generated by running the verifier with
/// --annotation_file and --extra_output_file
/// TODO: this is intermediate and probably doesn't need to be exposed (esp. serialized)
#[derive(Serialize, Deserialize, Debug)]
pub struct ProofAnnotations {
    pub annotation_file: PathBuf,
    pub extra_output_file: PathBuf,
}

#[cfg(test)]
mod tests {
    use crate::test_utils::load_test_case_file;
    use rstest::rstest;

    use super::*;

    /// Sanity check: verify that we can deserialize a public input JSON file.
    #[test]
    fn deserialize_public_input() {
        let public_input_str = load_test_case_file("fibonacci/air_public_input.json");
        let public_input: PublicInput = serde_json::from_str(&public_input_str)
            .expect("Failed to deserialize public input fixture");

        // We don't check all fields, just ensure that we can deserialize the fixture
        assert_eq!(public_input.layout, Layout::StarknetWithKeccak);
        assert_eq!(public_input.n_steps, 32768);
        assert_eq!(public_input.dynamic_params, None);
    }

    #[test]
    fn deserialize_solver_parameters() {
        let parameters_str = load_test_case_file("fibonacci/cpu_air_params.json");
        let parameters: ProverParameters = serde_json::from_str(&parameters_str)
            .expect("Failed to deserialize prover parameters fixture");

        // We don't check all fields, just ensure that we can deserialize the fixture
        assert!(!parameters.use_extension_field);
    }

    #[rstest]
    #[case("small", Layout::Small)]
    #[case("starknet_with_keccak", Layout::StarknetWithKeccak)]
    fn deserialize_layout(#[case] layout_str: String, #[case] expected: Layout) {
        let layout = Layout::from_str(&layout_str).unwrap();
        assert_eq!(layout, expected);
    }

    #[rstest]
    #[case(Layout::Small, "small")]
    #[case(Layout::StarknetWithKeccak, "starknet_with_keccak")]
    fn serialize_layout(#[case] layout: Layout, #[case] expected: &str) {
        let layout_str = layout.to_string();
        assert_eq!(layout_str, expected);
    }
}
