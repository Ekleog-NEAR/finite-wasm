use std::error;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::PathBuf;

#[path = "test.rs"]
mod test;

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("could not obtiain the current working directory")]
    CurrentDirectory(#[source] std::io::Error),
    #[error("could not walk the `tests` directory")]
    WalkDirEntry(#[source] walkdir::Error),
    #[error("could not write the test output to the standard error")]
    WriteTestOutput(#[source] std::io::Error),
    #[error("some tests failed")]
    TestsFailed,
}

struct Test {
    contents: String,
    path: PathBuf,
}

fn write_error(mut to: impl io::Write, error: impl error::Error) -> std::io::Result<()> {
    writeln!(to, "error: {}", error)?;
    let mut source = error.source();
    while let Some(error) = source {
        writeln!(to, "caused by: {}", error)?;
        source = error.source();
    }
    Ok(())
}

fn run() -> Result<(), Error> {
    let current_directory = std::env::current_dir().map_err(Error::CurrentDirectory)?;
    let tests_directory = current_directory.join("tests");
    let mut tests = Vec::new();
    for entry in walkdir::WalkDir::new(&tests_directory) {
        let entry = entry.map_err(Error::WalkDirEntry)?;
        if Some(OsStr::new("wast")) == entry.path().extension() {
            tests.push(Test {
                contents: String::new(),
                path: entry.path().into(),
            });
        }
    }

    println!("running {} tests", tests.len());
    let mut failures = 0;
    for test in &mut tests {
        let test_path = test
            .path
            .strip_prefix(&current_directory)
            .unwrap_or(&test.path);
        let test_name = test
            .path
            .strip_prefix(&tests_directory)
            .unwrap_or(&test.path);
        let mut context = test::TestContext::new(
            test_name.display().to_string(),
            test_path.into(),
            &test.contents,
        );
        context.run();
        if context.failed() {
            failures += 1;
        }
        std::io::stderr()
            .lock()
            .write_all(&context.output)
            .map_err(Error::WriteTestOutput)?;
    }

    if failures != 0 {
        Err(Error::TestsFailed)
    } else {
        Ok(())
    }
}

// Custom test harness
#[cfg(test)]
pub(crate) fn main() {
    std::process::exit(match run() {
        Ok(()) => 0,
        Err(error) => {
            write_error(std::io::stderr().lock(), &error).expect("failed writing out the error");
            1
        }
    })
}
