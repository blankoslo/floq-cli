// Working with subcommands is simple. There are a few key points to remember when working with
// subcommands in clap. First, they are really just Apps. This means they can have their own
// settings, version, authors, args, and even their own subcommands. The next thing to remember is
// that subcommands are set up in a tree like hierarchy.
//
// An ASCII art depiction may help explain this better. Using a fictional version of git as the demo
// subject. Imagine the following are all subcommands of git (note, the author is aware these aren't
// actually all subcommands in the real git interface, but it makes explanation easier)
//
//            Top Level App (git)                         TOP
//                           |
//    -----------------------------------------
//   /             |                \          \
// clone          push              add       commit      LEVEL 1
//   |           /    \            /    \       |
//  url      origin   remote    ref    name   message     LEVEL 2
//           /                  /\
//        path            remote  local                   LEVEL 3
//
// Given the above fictional subcommand hierarchy, valid runtime uses would be (not an all inclusive
// list):
//
// $ git clone url
// $ git push origin path
// $ git add ref local
// $ git commit message
//
// Notice only one command per "level" may be used. You could not, for example, do:
//
// $ git clone url push origin path
//
// It's also important to know that subcommands each have their own set of matches and may have args
// with the same name as other subcommands in a different part of the tree hierarchy (i.e. the arg
// names aren't in a flat namespace).
//
// In order to use subcommands in clap, you only need to know which subcommand you're at in your
// tree, and which args are defined on that subcommand.
//
// Let's make a quick program to illustrate. We'll be using the same example as above but for
// brevity sake we won't implement all of the subcommands, only a few.

use clap::{App, AppSettings, Arg};

fn main() {
    let matches = App::new("timetracker")
        .about("Timetracking in the terminal")
        .version("1.0")
        .author("The Rust Gang")
        .subcommand(
            App::new("clone")
                .about("clones repos")
                .arg(Arg::new("repo").about("The repo to clone").required(true)),
        )
        .subcommand(
            App::new("push")
                .about("pushes things")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    App::new("remote") // Subcommands can have their own subcommands,
                        // which in turn have their own subcommands
                        .about("pushes remote things")
                        .arg(
                            Arg::new("repo")
                                .required(true)
                                .about("The remote repo to push things to"),
                        ),
                )
                .subcommand(App::new("local").about("pushes local things")),
        )
        .subcommand(
            App::new("add")
                .about("adds things")
                .author("Someone Else") // Subcommands can list different authors
                .version("v2.0 (I'm versioned differently)") // or different version from their parents
                .setting(AppSettings::ArgRequiredElseHelp) // They can even have different settings
                .arg(
                    Arg::new("stuff")
                        .long("stuff")
                        .about("Stuff to add")
                        .takes_value(true)
                        .multiple(true),
                ),
        )
        .get_matches();

    // You can see which subcommand was used
    if let Some(subcommand) = matches.subcommand_name() {
        println!("'git {}' was used", subcommand);
    }

    // The most common way to handle subcommands is via a combined approach using
    // `ArgMatches::subcommand` which returns a tuple of both the name and matches
    match matches.subcommand() {
        ("clone", Some(clone_matches)) => {
            // Now we have a reference to clone's matches
            println!("Cloning {}", clone_matches.value_of("repo").unwrap());
        }
        ("push", Some(push_matches)) => {
            // Now we have a reference to push's matches
            match push_matches.subcommand() {
                ("remote", Some(remote_matches)) => {
                    // Now we have a reference to remote's matches
                    println!("Pushing to {}", remote_matches.value_of("repo").unwrap());
                }
                ("local", _) => {
                    println!("'git push local' was used");
                }
                _ => unreachable!(),
            }
        }
        ("add", Some(add_matches)) => {
            // Now we have a reference to add's matches
            println!(
                "Adding {}",
                add_matches
                    .values_of("stuff")
                    .unwrap()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        (_, None) => println!("No subcommand was used"), // If no subcommand was used it'll match the tuple ("", None)
        _ => unreachable!(), // If all subcommands are defined above, anything else is unreachabe!()
    }

    // Continued program logic goes here...
}
