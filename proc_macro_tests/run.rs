// Copyright (c) 2019 King's College London created by the Software Development Team
// <http://soft-dev.org/>
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0>, or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, or the UPL-1.0 license <http://opensource.org/licenses/UPL>
// at your option. This file may not be copied, modified, or distributed except according to those
// terms.

use std::{fs::read_dir, path::PathBuf, process::Command};

use lang_tester::LangTester;
use tempdir::TempDir;

#[cfg(debug_assertions)]
const DEPS_PATH: &str = "target/debug/deps";
#[cfg(not(debug_assertions))]
const DEPS_PATH: &str = "target/release/deps";

/// Fish out libnatrob.so from the target/ directory.
fn natrob_lib() -> String {
    let mut cnds = Vec::new();
    for e in read_dir(DEPS_PATH).unwrap() {
        let path = e.unwrap().path();
        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("libnatrob")
            && path.extension().map(|x| x.to_str().unwrap()) == Some("so")
        {
            cnds.push(path.to_str().unwrap().to_owned());
        }
    }
    if cnds.is_empty() {
        panic!("Can't find libnatrob.so");
    } else if cnds.len() == 1 {
        return cnds[0].clone();
    } else {
        panic!("Multiple candidates for libnatrob.so");
    }
}

fn main() {
    let tempdir = TempDir::new("proc_macro_tests").unwrap();
    let natrob_lib = natrob_lib();
    LangTester::new()
        .test_dir("proc_macro_tests")
        // Only use files named `test/*.rs` as test files.
        .test_file_filter(|p| {
            p.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("test_")
                && p.extension().and_then(|s| Some(s.to_str().unwrap() == "rs")).unwrap_or(false)
        })
        // Extract the first sequence of commented line(s) as the tests.
        .test_extract(|s| {
            Some(
                s.lines()
                    // Skip non-commented lines at the start of the file.
                    .skip_while(|l| !l.starts_with("//"))
                    // Extract consecutive commented lines.
                    .take_while(|l| l.starts_with("//"))
                    .map(|l| &l[2..])
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        })
        // We have two test commands:
        //   * `Compiler`: runs rustc.
        //   * `Run-time`: if rustc does not error, and the `Compiler` tests
        //     succeed, then the output binary is run.
        .test_cmds(move |p| {
            // Test command 1: Compile `x.rs` into `tempdir/x`.
            let mut exe = PathBuf::new();
            exe.push(&tempdir);
            exe.push(p.file_stem().unwrap());
            let mut compiler = Command::new("rustc");
            compiler.args(&[
                "--extern",
                &format!("natrob={}", natrob_lib),
                "-o",
                exe.to_str().unwrap(),
                p.to_str().unwrap(),
            ]);
            // Test command 2: run `tempdir/x`.
            let runtime = Command::new(exe);
            vec![("Compiler", compiler), ("Run-time", runtime)]
        })
        .run();
}
