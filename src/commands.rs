use std::path::Path;

use crate::gui::RenderContext;

const CMDS: &[&str] = &["exec", "pwd", "cd", "quit"];

fn possible_command(unknown: &str) -> Option<&str> {
    let mut distance = u32::MAX;
    let mut best_guess = "";
    for cmd in CMDS {
        let d = triple_accel::levenshtein_exp(unknown.as_bytes(), cmd.as_bytes());
        if d < distance {
            distance = d;
            best_guess = cmd;
        }
    }

    // A guess that's less than 2 `steps` away from a correct arg.
    (distance <= 2).then_some(best_guess)
}

fn print_cwd(ctx: &mut RenderContext) {
    match std::env::current_dir() {
        Ok(path) => ctx
            .terminal_prompt
            .push_str(&format!("Working directory {}.\n", path.display())),
        Err(err) => ctx.terminal_prompt.push_str(&format!("Failed to print pwd: '{err}'\n")),
    }
}

fn expand_homedir<P: AsRef<Path>>(path: P) -> std::path::PathBuf {
    let path = path.as_ref();

    if !path.starts_with("~") {
        return path.to_path_buf();
    }

    let mut home_dir = match dirs::home_dir() {
        Some(dir) => dir,
        None => return path.to_path_buf(),
    };

    if path == Path::new("~") {
        return home_dir;
    }

    if home_dir == Path::new("/") {
        // Corner case: `home_dir` root directory;
        // don't prepend extra `/`, just drop the tilde.
        path.strip_prefix("~").unwrap().to_path_buf()
    } else {
        home_dir.push(path.strip_prefix("~/").unwrap());
        home_dir
    }
}

pub fn process_commands(ctx: &mut RenderContext, commands: &[String]) {
    for cmd in commands {
        ctx.terminal_prompt.push_str(&format!("(bite) {cmd}\n"));

        let mut args = cmd.split_whitespace();
        let cmd_name = match args.next() {
            Some(cmd) => cmd,
            None => continue,
        };

        if cmd_name == "exec" || cmd_name == "e" {
            if let Some(unexpanded) = args.next() {
                let path = expand_homedir(unexpanded);

                ctx.start_disassembling(path);
                ctx.terminal_prompt.push_str(&format!("Binary '{unexpanded}' was opened.\n"));
                continue;
            }

            ctx.terminal_prompt.push_str(&format!("Command 'exec' requires a path.\n"));
            continue;
        }

        if cmd_name == "cd" {
            let path = expand_homedir(args.next().unwrap_or("~"));

            if let Err(err) = std::env::set_current_dir(path) {
                ctx.terminal_prompt.push_str(&format!("Failed to change directory: '{err}'.\n"));
                continue;
            }

            print_cwd(ctx);
            continue;
        }

        if cmd_name == "pwd" {
            print_cwd(ctx);
            continue;
        }

        if cmd_name == "r" || cmd_name == "run" {
            let mut args: Vec<&str> = Vec::new();

            if let Some((_, raw_args)) = cmd.split_once("--") {
                args = raw_args.split_whitespace().collect();
            }

            let path = match ctx.process_path {
                Some(ref path) => path,
                None => {
                    ctx.terminal_prompt.push_str("There are no targets to run.\n");
                    continue;
                }
            };

            ctx.start_debugging(path.to_path_buf(), &args);
            continue;
        }

        if cmd_name == "quit" || cmd_name == "q" {
            std::process::exit(0);
        }

        match possible_command(cmd_name) {
            Some(guess) => ctx.terminal_prompt.push_str(&format!(
                "Command '{cmd}' is unknown, did you mean '{guess}'?.\n"
            )),
            None => ctx.terminal_prompt.push_str(&format!("Command '{cmd}' is unknown.\n")),
        }
    }
}