//! `aindb` — run a node, submit requests, and inspect history.
//!
//! Planned subcommands (SPEC §7.3, ROADMAP):
//!   serve    --data DIR --socket PATH        (M5)
//!   submit   --json FILE                     (M2)
//!   explain  tx <id> | object <id>           (M3)
//!   history  object <id> | principal <id>    (M3)
//!   verify   [--state] [--anchors]           (M3)
//!   checkpoint                               (M3)

const USAGE: &str = "usage: aindb <serve|submit|explain|history|verify|checkpoint> [args]\n\
                     AINDB v0.0.1 — specification stage, commands not implemented yet.";

fn main() {
    let cmd = std::env::args().nth(1);
    match cmd.as_deref() {
        None | Some("-h") | Some("--help") => println!("{USAGE}"),
        Some(c @ ("serve" | "submit" | "explain" | "history" | "verify" | "checkpoint")) => {
            eprintln!("aindb {c}: not implemented yet (see ROADMAP.md)");
            std::process::exit(2);
        }
        Some(other) => {
            eprintln!("aindb: unknown command '{other}'\n{USAGE}");
            std::process::exit(2);
        }
    }
}
