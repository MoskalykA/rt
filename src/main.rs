use argh::FromArgs;
use log::{error, info, LevelFilter};
use phf::phf_map;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::*,
    thread,
};

#[derive(FromArgs)]
/// A tool to facilitate these projects
struct Args {
    /// command
    #[argh(option, default = "String::from(\"dev\")", short = 'c')]
    command: String,

    /// project
    #[argh(option, short = 'p')]
    project: Option<String>,

    /// file name
    #[argh(option, default = "String::from(\"rt.yaml\")", short = 'f')]
    file_name: String,
}

#[derive(Deserialize, Clone)]
struct Task {
    platform: String,
    commands: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct Options {
    tasks: Option<HashMap<String, Task>>,
    files: Option<Vec<String>>,
}

fn read_file(file_name: String, register: &mut HashMap<String, Vec<Commands>>) {
    let mut path = env::current_dir().unwrap();
    path = path.join(file_name);

    let path_without_extension = path
        .with_extension("")
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();

    if Path::exists(&path) {
        let content = fs::read_to_string(path.clone()).unwrap();
        let options: Options = serde_yaml::from_str(&content).unwrap();
        if let Some(files) = options.files {
            for file in files {
                read_file(file, register);
            }
        }

        if let Some(tasks) = options.tasks {
            for (name, task) in tasks {
                if let Some(register_commands) = register.get_mut(&name) {
                    register_commands.push(Commands {
                        path: path.clone(),
                        task,
                        project: path_without_extension.clone(),
                    });
                } else {
                    register.insert(
                        name,
                        vec![Commands {
                            path: path.clone(),
                            task,
                            project: path_without_extension.clone(),
                        }],
                    );
                }
            }
        }

        info!("The `{}` file has just been interpreted", path.display());
    } else {
        error!("The file `{}` cannot be found", path.display());

        exit(0x0100);
    }
}

#[derive(Clone)]
struct Commands {
    pub path: PathBuf,
    pub task: Task,
    pub project: String,
}

fn has_program(program: &str) -> bool {
    Command::new("which")
        .arg(program)
        .status()
        .unwrap()
        .success()
}

static PLATFORMS: phf::Map<&'static str, (&'static str, &'static str)> = phf_map! {
    "npm" => (if cfg!(windows) {
        "npm.cmd"
    } else {
        "npm"
    }, "exec"),
    "pnpm" => ("pnpm", "exec"),
    "yarn" => (if cfg!(windows) {
        "yarn.cmd"
    } else {
        "yarn"
    }, "run"),
    "cargo" => ("cargo", "run")
};

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
        let commands: Vec<Commands> = if let Some(project) = args.project {
            commands
                .iter()
                .filter(|command| command.project == project)
                .cloned()
                .collect()
        } else {
            commands.to_vec()
        };

        for command in commands {
            s.spawn(move || {
                if !has_program(&command.task.platform) {
                    error!(
                        "The `{}` program must be installed on your computer",
                        command.task.platform
                    );

                    exit(0x0100);
                }

                let base = if let Some((base, _)) = PLATFORMS.get(&command.task.platform) {
                    base.to_string()
                } else {
                    error!(
                        "The `{}` platform you specified is not available",
                        command.task.platform
                    );

                    exit(0x0100);
                };

                let perform = if let Some((_, perform)) = PLATFORMS.get(&command.task.platform) {
                    perform.to_string()
                } else {
                    error!(
                        "The `{}` platform you specified is not available",
                        command.task.platform
                    );

                    exit(0x0100);
                };

                info!(
                    "Running a command from group `{group}` (`{}`)",
                    if let Some(commands) = &command.task.commands {
                        vec![base.clone(), perform.clone(), commands.join(" ")].join(" ")
                    } else {
                        vec![base.clone(), perform.clone()].join(" ")
                    }
                );

                let mut process = Command::new(&base);
                process.stdout(Stdio::piped()).arg(&perform);

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
