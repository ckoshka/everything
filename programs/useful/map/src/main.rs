use cmd_lib::*;
use crossbeam::queue::SegQueue;
use crossbeam::thread;
use term_macros::*;

fn main() {
    tool! {
        args:
            - cmd: String;
                ? !cmd.contains("{file}")
                => "needs to contain {file} somewhere so i can interpolate a  filename into the command safely"

            - input_dir: String;
                ? !std::path::Path::new(&input_dir).exists()
                => "the path you entered for input_dir doesn't exist"

            - output_dir: String;

            - concurrency: usize = 4;
        ;

        body: || {

            let q = SegQueue::new();
            std::fs::read_dir(&input_dir)
                .expect("unable to read directory")
                .map(|f| f.map(|file| file.path())
                    .map(|path| format!("{} > {}/{}", cmd.replace("{file}", &path.as_os_str().to_string_lossy()), output_dir, path.file_name().unwrap().to_str().unwrap()))
                )
                .for_each(|path| {
                    let _ = path.map(|p| q.push(p));
                });

            run_cmd!(mkdir $output_dir).expect("Creating the directory didn't work");

            thread::scope(|s| {
                for _ in 0..concurrency {
                    s.spawn(|_| {
                        loop {
                            let cmd_task = q.pop().map(|task| {
                                let _ = exec_command(task);
                            });
                            if cmd_task.is_none() {
                                break;
                            }
                        }
                    });
                }
            }).unwrap();


        }
    }
}

fn exec_command(cmd: String) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Running {}", cmd);
    run_cmd!(bash -c $cmd)?;
    Ok(())
}
