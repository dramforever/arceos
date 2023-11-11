use fdt_iter::*;
use std::{env::args, error, fs::File, io::Read, process::exit};

fn dump_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<String>>()
        .join(" ")
}

fn recursive_print(mut iter: Iter, depth: usize) {
    let indent = "    ".repeat(depth);
    let node = iter.node();

    println!(
        "{indent:}{name:} {{ // {offset:}",
        name = node.name().to_string_lossy(),
        offset = node.offset()
    );

    for (name, value) in node.properties() {
        println!(
            "{indent:}    {name:} = [{value}];",
            name = name.to_string_lossy(),
            value = dump_bytes(value),
        );
    }

    while let Some(child) = iter.next_child() {
        recursive_print(child, depth + 1);
    }

    println!("{indent:}}};");
}

fn main() -> Result<(), Box<dyn error::Error>> {
    let file = match &args().collect::<Vec<_>>()[..] {
        [_argv0, file] => file.clone(),
        [argv0, ..] => {
            eprintln!("Usage: {argv0:} <file.dtb>");
            exit(1)
        }
        [] => panic!("No args"),
    };
    let mut bytes: Vec<u8> = Vec::new();
    File::open(file)?.read_to_end(&mut bytes)?;
    let fdt = Fdt::from_bytes(&bytes).map_err(|err| format!("{err:?}"))?;
    println!("/dts-v1/;");
    println!();
    print!("/");
    recursive_print(fdt.root().walker().iter(), 0);
    Ok(())
}
