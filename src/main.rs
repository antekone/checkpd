extern crate serialize;
extern crate docopt;

use docopt::Docopt;
use std::io::{BufferedReader,File};
use std::slice::bytes::MutableByteVector;

static USAGE: &'static str = "
Usage: xorfile [options] [-n NAME ...]

Options:
    -h, --help                  Show this helpful screen
    -r SECTOR, --resume SECTOR  Resume from this sector
    -n NAME, --name NAME        Filename to read. Can (should) occur multiple times.

Example:

    xorfile -n file1.bin -n file2.bin -n file3.bin -r 512

    This will check if PD of selected files is the same. Starts from sector 512.
";

#[deriving(Decodable)]
struct Args {
    flag_name: Vec<String>,
    flag_resume: Option<u64>,
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|dopt| dopt.decode()).unwrap_or_else(|e| e.exit());

    let resume_sector = args.flag_resume.unwrap_or(0);
    println!("Resuming from sector {}.", resume_sector);

    calculate_pd(args.flag_name, resume_sector);
}

fn calculate_pd(filenames: Vec<String>, resume_from: u64) {
    if filenames.len() == 0 {
        println!("Use --help for help.");
        return;
    }

    let mut readers: Vec<BufferedReader<File>> = Vec::new();
    println!("Opening {} streams.", filenames.len());
    for fname in filenames.iter() {
        let mut file = match File::open(&Path::new(fname)) {
            Ok(f) => f,
            Err(e) => {
                println!("Can't open file: {}", fname);
                println!("Reason: {}", e);
                return;
            }
        };

        match file.seek(512 * resume_from as i64, std::io::SeekStyle::SeekSet) {
            Ok(_) => {}
            Err(f) => {
                println!("I/O error during resuming: {}", f);
                return;
            }
        };

        readers.push(BufferedReader::new(file));
    }

    const BUF_SIZE: uint = 10 * 1024 * 1024;

    let mut offs: u64 = 0;
    let mut xorbuf = box [0, ..BUF_SIZE];
    let mut buf = box [0, ..BUF_SIZE];

    loop {
        xorbuf.as_mut_slice().set_memory(0);

        for i in range(0, filenames.len()) {
            let mut rd = &mut readers[i];

            let read = match rd.read(buf.as_mut_slice()) {
                Ok(read) if read == BUF_SIZE => read,
                _ => {
                    println!("I/O error when reading. End of file?");
                    return;
                }
            };

            if read != BUF_SIZE {
                println!("I/O error?");
                return;
            }

            for ofs in range(0, BUF_SIZE) {
                xorbuf[ofs] ^= buf[ofs];
            }
        }

        let mut pd_ok = true;
        let mut bad_offs = 0i;
        for i in range(0, BUF_SIZE) {
            if xorbuf[i] != 0 {
                bad_offs = i as int;
                pd_ok = false;
                break;
            }
        }

        if offs % (100 * 1024 * 1024) == 0 {
            println!("... offset {:016x} ({} mb) ...", offs, offs / 1024 / 1024);
        }

        if ! pd_ok {
            println!("bad pd {:016x}", offs + bad_offs as u64);
        }

        offs += BUF_SIZE as u64;
    }
}

