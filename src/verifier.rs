use std::path::Path;

use crate::error::VerifierError;
use crate::models::ProofAnnotations;

/// Run the Stone Verifier on the specified program execution, asynchronously.
///
/// The main difference from the synchronous implementation is that the verifier process
/// is spawned asynchronously using `tokio::process::Command`.
///
/// This function abstracts the method used to call the verifier. At the moment we invoke
/// the verifier as a subprocess but other methods can be implemented (ex: FFI).
///
/// * `in_file`: Path to the proof generated from the prover. Corresponds to its "--out-file".
pub fn run_verifier(in_file: &Path) -> Result<(), VerifierError> {
    run_verifier_from_command_line(in_file, None, None)
}

/// Run the Stone Verifier on the specified program execution.
///
/// This function abstracts the method used to call the verifier. At the moment we invoke
/// the verifier as a subprocess but other methods can be implemented (ex: FFI).
///
/// * `in_file`: Path to the proof generated from the prover. Corresponds to its "--out-file".
/// * `annotation_file`: Path to the annotations file, which will be generated as output.
/// * `extra_output_file`: Path to the extra annotations file, which will be generated as output.
pub fn run_verifier_with_annotations(
    in_file: &Path,
    annotation_file: &Path,
    extra_output_file: &Path,
) -> Result<(), VerifierError> {
    run_verifier_from_command_line(in_file, Some(annotation_file), Some(extra_output_file))
}

/// Call the Stone Verifier from the command line, asynchronously.
///
/// Input files must be prepared by the caller.
///
/// * `in_file`: Path to the proof generated from the prover. Corresponds to its "--out-file".
/// * `annotation_file`: Path to the annotations file, which will be generated as output.
/// * `extra_output_file`: Path to the extra annotations file, which will be generated as output.
pub fn run_verifier_from_command_line(
    in_file: &Path,
    annotation_file: Option<&Path>,
    extra_output_file: Option<&Path>,
) -> Result<(), VerifierError> {
    let mut command = std::process::Command::new("cpu_air_verifier");
    command
        .arg("cpu_air_verifier")
        .arg("--in_file")
        .arg(in_file);

    if let Some(annotation_file) = annotation_file {
        command.arg("--annotation_file").arg(annotation_file);
    }

    if let Some(extra_output_file) = extra_output_file {
        command.arg("--extra_output_file").arg(extra_output_file);
    }

    let output = command.output()?;

    if !output.status.success() {
        return Err(VerifierError::CommandError(output));
    }

    Ok(())
}

/// Run the Stone Verifier on the specified program execution, asynchronously.
///
/// The main difference from the synchronous implementation is that the verifier process
/// is spawned asynchronously using `tokio::process::Command`.
///
/// This function abstracts the method used to call the verifier. At the moment we invoke
/// the verifier as a subprocess but other methods can be implemented (ex: FFI).
///
/// * `in_file`: Path to the proof generated from the prover. Corresponds to its "--out-file".
pub async fn run_verifier_async(in_file: &Path) -> Result<(), VerifierError> {
    run_verifier_from_command_line_async(in_file, None, None).await
}

/// Run the Stone Verifier on the specified program execution, asynchronously.
///
/// The main difference from the synchronous implementation is that the verifier process
/// is spawned asynchronously using `tokio::process::Command`.
///
/// This function abstracts the method used to call the verifier. At the moment we invoke
/// the verifier as a subprocess but other methods can be implemented (ex: FFI).
///
/// * `in_file`: Path to the proof generated from the prover. Corresponds to its "--out-file".
/// * `annotation_file`: Path to the annotations file, which will be generated as output.
/// * `extra_output_file`: Path to the extra annotations file, which will be generated as output.
pub async fn run_verifier_with_annotations_async(
    in_file: &Path,
    annotation_file: &Path,
    extra_output_file: &Path,
) -> Result<ProofAnnotations, VerifierError> {
    run_verifier_from_command_line_async(in_file, Some(annotation_file), Some(extra_output_file))
        .await?;

    let annotations = ProofAnnotations {
        annotation_file: annotation_file.into(),
        extra_output_file: extra_output_file.into(),
    };
    Ok(annotations)
}

/// Call the Stone Verifier from the command line, asynchronously.
///
/// Input files must be prepared by the caller.
///
/// * `in_file`: Path to the proof generated from the prover. Corresponds to its "--out-file".
/// * `annotation_file`: Path to the annotations file, which will be generated as output.
/// * `extra_output_file`: Path to the extra annotations file, which will be generated as output.
pub async fn run_verifier_from_command_line_async(
    in_file: &Path,
    annotation_file: Option<&Path>,
    extra_output_file: Option<&Path>,
) -> Result<(), VerifierError> {
    let mut command = tokio::process::Command::new("cpu_air_verifier");
    command
        .arg("cpu_air_verifier")
        .arg("--in_file")
        .arg(in_file);

    if let Some(annotation_file) = annotation_file {
        command.arg("--annotation_file").arg(annotation_file);
    }

    if let Some(extra_output_file) = extra_output_file {
        command.arg("--extra_output_file").arg(extra_output_file);
    }

    let output = command.output().await?;

    if !output.status.success() {
        return Err(VerifierError::CommandError(output));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::test_utils::{prover_in_path, prover_test_case, ProverTestCase};

    use super::*;

    /// Check that the Stone Verifier command-line wrapper works.
    #[rstest]
    fn test_run_verifier_from_command_line(
        prover_test_case: ProverTestCase,
        #[from(prover_in_path)] _path: (),
    ) {
        let proof_file = prover_test_case.proof_file;
        run_verifier_from_command_line(proof_file.as_path(), None, None)
            .expect("Proof file is valid");
    }

    #[rstest]
    fn test_run_verifier(prover_test_case: ProverTestCase, #[from(prover_in_path)] _path: ()) {
        let proof_file = prover_test_case.proof_file;
        run_verifier(proof_file.as_path()).expect("Proof file is valid");
    }

    #[rstest]
    fn test_run_verifier_with_annotations(
        prover_test_case: ProverTestCase,
        #[from(prover_in_path)] _path: (),
    ) {
        let output_dir = tempfile::tempdir().unwrap();
        let annotation_file = output_dir.path().join("annotations.json");
        let extra_output_file = output_dir.path().join("extra_output_file.json");

        run_verifier_with_annotations(
            prover_test_case.proof_file.as_path(),
            annotation_file.as_path(),
            extra_output_file.as_path(),
        )
        .expect("Proof is valid");
        // TODO: generate fixtures to compare the generated files
        assert!(annotation_file.exists());
        assert!(extra_output_file.exists());
    }

    /// Check that the Stone Verifier command-line wrapper works.
    #[rstest]
    #[tokio::test]
    async fn test_run_verifier_from_command_line_async(
        prover_test_case: ProverTestCase,
        #[from(prover_in_path)] _path: (),
    ) {
        let proof_file = prover_test_case.proof_file;
        run_verifier_from_command_line_async(proof_file.as_path(), None, None)
            .await
            .expect("Proof file is valid");
    }

    #[rstest]
    #[tokio::test]
    async fn test_run_verifier_async(
        prover_test_case: ProverTestCase,
        #[from(prover_in_path)] _path: (),
    ) {
        let proof_file = prover_test_case.proof_file;
        run_verifier_async(proof_file.as_path())
            .await
            .expect("Proof file is valid");
    }

    #[rstest]
    #[tokio::test]
    async fn test_run_verifier_with_annotations_async(
        prover_test_case: ProverTestCase,
        #[from(prover_in_path)] _path: (),
    ) {
        let output_dir = tempfile::tempdir().unwrap();
        let annotation_file = output_dir.path().join("annotations.json");
        let extra_output_file = output_dir.path().join("extra_output_file.json");

        run_verifier_with_annotations_async(
            prover_test_case.proof_file.as_path(),
            annotation_file.as_path(),
            extra_output_file.as_path(),
        )
        .await
        .expect("Proof is valid");
        // TODO: generate fixtures to compare the generated files
        assert!(annotation_file.exists());
        assert!(extra_output_file.exists());
    }
}
