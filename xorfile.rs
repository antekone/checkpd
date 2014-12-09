use getopts::{optmulti,optflag,getopts,optopt};
use std::os;
use std::io::{BufferedReader,File};
use std::num::from_str_radix;

fn syntax_error(pname: &str, f: getopts::Fail_) {
    println!("Syntax error in arguments.");
    println!("{}", f);
    syntax(pname);
}

fn syntax(pname: &str) {
    println!("Syntax:");
    println!("");
    println!("    {} <options...>", pname);
    println!("");
    println!("Options:");
    println!("");
    println!("    -h        this screen");
    println!("    -r SECTOR resume from sector SECTOR");
    println!("    -n NAME   filename to read. can occur multiple times.");
    println!("");
    println!("Example:");
    println!("");
    println!("    {} -n file1.bin -n file2.bin -n file3.bin -r 512", pname);
    println!("");
    println!("    This will check if PD of selected files is the same. Start from sector 512.");
}

fn main() {
    let args = os::args();
    let program = args[0].as_slice();
    let args = args.tail();

    let opts = [
        optmulti("n", "name", "filename", "FILE"),
        optopt("r", "resume", "resume sector number", "SECTOR"),
        optflag("h", "help", "this screen"),
    ];

    let matches = match getopts(args, &opts) {
        Ok(m) => m,
        Err(f) => {
            syntax_error(program, f);
            return;
        }
    };

    if matches.opt_present("h") {
        syntax(program);
        return;
    }

    let resume_sector = match matches.opt_present("r") {
        true => {
            let argstr: String = matches.opt_str("r").unwrap_or("0".to_string());
            let num: u64 = std::num::from_str_radix(matches.opt_str("r").unwrap().as_slice(), 10).unwrap_or(0);
            num
        },
        false => 0,
    };

    println!("Resuming from sector {}.", resume_sector);

    let files = matches.opt_strs("n");
    calculate_pd(files, resume_sector);
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

        file.seek(512 * resume_from as i64, std::io::SeekStyle::SeekSet);
        readers.push(BufferedReader::new(file));
    }

    const BUF_SIZE: uint = 10 * 1024 * 1024;

    let mut offs: u64 = 0;
    let mut xorbuf = box [0, ..BUF_SIZE];
    let mut buf = box [0, ..BUF_SIZE];

    while true {
        for i in range(0, BUF_SIZE) {
            xorbuf[i] = 0;
        }

        for i in range(0, filenames.len()) {
            let mut rd = &mut readers[i];

            let read = match rd.read(buf.as_mut_slice()) {
                Ok(read) => read,
                Err(f) => {
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

