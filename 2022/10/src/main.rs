use color_eyre::{eyre::Context, Report};

mod virtual_machine;

#[derive(Debug, Clone)]
pub enum Instruction {
    Noop,
    Addx(i32),
}

fn parse_program(file_contents: &str) -> Result<Vec<Instruction>, Report> {

    file_contents.lines().filter(|line| {
        !line.is_empty()
    }).map(|line| {
        
        use Instruction::*;
        use nom::{branch::alt, bytes::complete::tag, sequence::preceded, Parser};

        // Nightmare
        Ok(alt::<_, _, (_, nom::error::ErrorKind), _>((
            tag("noop").map(|_| Noop),
            preceded(tag("addx "), nom::character::complete::i32).map(Addx),
        ))(line).map_err(|e| e.to_owned()).context("Failed to parse program")?.1)

    }).collect()
}

fn main() -> Result<(), Report> {

    color_eyre::install().unwrap();

    let (_app_name, args) = {
        let mut args_iter = std::env::args();
        (args_iter.next().expect("argv[0] is not application name"), args_iter.collect::<Vec<_>>())
    };

    if args.is_empty() {
        eprintln!("Error: no file name was provided.");
        std::process::exit(1);
    }

    let file_contents = std::fs::read_to_string(&args[0]).context("Unable to read input file")?;

    let instructions = parse_program(&file_contents)?;

    async fn interpret_instruction(cpu: &std::sync::RwLock<virtual_machine::Vm>, instruction: Instruction) {
        
        match instruction {
            Instruction::Noop => {
                virtual_machine::yield_cycles(1).await;
            },
            Instruction::Addx(x) => {
                virtual_machine::yield_cycles(2).await;
                cpu.write().unwrap().reg_x += x;
            },
        }
    }

    const FB_WIDTH: usize = 40;
    const FB_HEIGHT: usize = 6;

    let mut frame_buffer = [['.'; FB_WIDTH]; FB_HEIGHT];

    virtual_machine::Vm::execute(&instructions, interpret_instruction, |cpu| {
        let cycle = cpu.get_cycle() + 1; // AOC cycles are 1- indexed.

        if cycle as usize >= FB_WIDTH * FB_HEIGHT {
            return;
        }

        let to_coords = |idx: i32| -> (i32, i32) {

            const WIDTH_I32: i32 = FB_WIDTH as i32;

            let mut x: i32 = idx;
            let mut y: i32 = 0;
            while x >= WIDTH_I32 {
                y += 1;
                x -= WIDTH_I32;
            }
    
            if y >= (FB_HEIGHT as i32) {
                eprintln!("Y out of bounds @ cycle {cycle}");
            }
            (x, y)
        };

        let sprite = {

            let (x, _) = to_coords(cpu.reg_x);

            (x - 1)..=(x + 1)
        };

        let (beam_x, beam_y) = to_coords(cycle as i32 - 1);

        if sprite.contains(&beam_x) {
            frame_buffer[beam_y as usize][beam_x as usize] = '#';
        }

    })?;

    for (i, row) in frame_buffer.into_iter().enumerate() {
        println!("Cycle {: >3} -> [{}] <- Cycle {: >3}", i * row.len() + 1, String::from_iter(row), (i + 1) * row.len());
    }

    Ok(())
}