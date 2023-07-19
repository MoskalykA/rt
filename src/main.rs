use argh::FromArgs;
use log::{error, info, LevelFilter};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::*,
    thread,
};

#[derive(FromArgs)]
/// Maybe
struct Args {
    /// command
    #[argh(option, default = "String::from(\"dev\")", short = 'c')]
    command: String,

    /// file name
    #[argh(option, default = "String::from(\"rt.yaml\")", short = 'f')]
    file_name: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Task {
    platform: String,
    commands: Option<Vec<String>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Options {
    tasks: HashMap<String, Task>,
    files: Option<Vec<String>>,
}

fn read_file(file_name: String, register: &mut HashMap<String, Vec<Commands>>) {
    let mut path = env::current_dir().unwrap();
    path = path.join(file_name);

    if Path::exists(&path) {
        let content = fs::read_to_string(path.clone()).unwrap();
        let options: Options = serde_yaml::from_str(&content).unwrap();
        if let Some(files) = options.files {
            for file in files {
                read_file(file, register);
            }
        }

        for (name, task) in options.tasks {
            if let Some(register_commands) = register.get_mut(&name) {
                register_commands.push(Commands {
                    path: path.clone(),
                    task,
                });
            } else {
                register.insert(
                    name,
                    vec![Commands {
                        path: path.clone(),
                        task,
                    }],
                );
            }
        }

        info!("The `{}` file has just been interpreted", path.display());
    } else {
        error!("The file `{}` cannot be found", path.display());

        exit(0x0100);
    }
}

#[derive(Debug)]
struct Commands {
    pub path: PathBuf,
    pub task: Task,
}

fn has_program(program: &str) -> bool {
    let paths = env::var("PATH").unwrap();
    let path = paths.split(';').find(|path| path.contains(program));
    path.is_some()
}

fn main() {
    env_logger::Builder::new()
        .filter_level(LevelFilter::max())
        .init();

    let mut commands: HashMap<String, Vec<Commands>> = HashMap::new();
    let args: Args = argh::from_env();
    read_file(args.file_name, &mut commands);

    let group = &args.command;
    let commands = commands.get(&args.command).unwrap();
    thread::scope(|s| {
        for command in commands {
            if !has_program(&command.task.platform) {
                error!(
                    "The `{}` program must be installed on your computer",
                    command.task.platform
                );

                exit(0x0100);
            }

            s.spawn(move || {
                let first_command = String::from(if command.task.platform == "pnpm" {
                    "exec"
                } else {
                    "run"
                });

                let full_command = if let Some(commands) = &command.task.commands {
                    vec![
                        command.task.platform.clone(),
                        first_command.clone(),
                        commands.join(" "),
                    ]
                    .join(" ")
                } else {
                    vec![command.task.platform.clone(), first_command.clone()].join(" ")
                };

                info!("Running a command from group `{group}` (`{full_command}`)");

                let mut process = Command::new(&command.task.platform);
                process.stdout(Stdio::piped()).arg(&first_command);

                if let Some(commands) = &command.task.commands {
                    process.args(commands);
                }

                process.current_dir(command.path.parent().unwrap());

                let mut child = process.spawn().expect("command failed to start");
                let stdout = child.stdout.as_mut().unwrap();
                let stdout_reader = BufReader::new(stdout);
                let stdout_lines = stdout_reader.lines();
                for line in stdout_lines {
                    println!("{}", line.unwrap());
                }

                child.wait().unwrap();
            });
        }
    });
}
