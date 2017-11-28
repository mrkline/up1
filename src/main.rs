extern crate getopts;

use getopts::Options;

use std::env;
use std::process::exit;
use std::path::{Path, PathBuf};
use std::fs;
use std::io;

// Print our usage string and exit the program with the given code.
// (This never returns.)
fn print_usage(opts: &Options, code: i32) -> ! {
    println!("{}", opts.usage("Usage: up1 [options] <directory>"));
    exit(code);
}

fn only_entry_in_parent(child: &Path, parent: &Path) -> io::Result<bool> {
    let mut target_iter = fs::read_dir(&parent)?;

    let first_entry = match target_iter.next() {
        Some(c) => c?,
        None => {
            // Invariant: The parent contains no entries
            // (it should at least contain child).
            panic!("The parent directory contains no child entries");
        }
    };

    if target_iter.next().is_some() {
        return Ok(false);
    }

    if first_entry.path() != child {
        // Invariant: The parent does not contain the child
        panic!("The parent directory ({}) does not contain the child ({})",
               parent.display(), child.display());
    }
    Ok(true)
}


fn get_unique_temporary_name(child_dir : &Path, parent_dir : &Path) -> PathBuf {
    // We're replacing parent_dir with child_dir, so find _its_ parent
    PathBuf::new()
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut opts = Options::new();
    opts.optflag("h", "help", "Print this help text.");
    opts.optflag("n", "dry-run", "Show the commands which would be run.");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(e) => {
            // If the user messes up the args, print the error and usage string.
            eprintln!("{}", e.to_string());
            print_usage(&opts, 1);
        }
    };

    if matches.opt_present("h") { // Print help as-desired.
        print_usage(&opts, 0);
    }

    if matches.free.len() > 1 { print_usage(&opts, 1); }

    let target_dir = match matches.free.len() {
        0 => env::current_dir().expect("Couldn't get current directory"),
        1 => PathBuf::from(&matches.free[0]),
        _ => unreachable!()
    };

    // Cannonicalize these to make life a touch more straightforward:
    let target_dir = match fs::canonicalize(&target_dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Couldn't canonicalize {}: {}", target_dir.display(), e.to_string());
            exit(1);
        }
    };
    let parent_dir = match target_dir.parent() {
        Some(p) => p,
        None => {
            eprintln!("{} has no parent.", target_dir.display());
            exit(1);
        }
    };

    if !only_entry_in_parent(&target_dir, &parent_dir).unwrap() {
        eprintln!("{} has more entries than just {}", parent_dir.display(), target_dir.display());
        exit(1);
    }

    let dry_run = matches.opt_present("n");

    if dry_run {
        println!("Moving the contents of {}", target_dir.display());
        println!("to: {}", parent_dir.display());
        exit(0);
    }
}
