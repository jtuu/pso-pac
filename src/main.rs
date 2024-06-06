use std::env;
use std::fs;
use std::io::Write;
use std::mem::size_of;

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct PacItemHeader {
    unk1: u16, // id?
    unk2: u16, // sub id?
    unk3: u32,
    sample_rate: u32,
    sample_size: u16,
    bit_depth: u16, // sample bits?
    magic: [u8; 4],
    size1: u32,
    unk6: u16,
    unk7: u16,
    size2: u32, // next item at aligned to 0x20
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct WavHeader {
    magic_riff: [u8; 4],
    wave_size: u32,
    magic_wave: [u8; 4],
    magic_fmt: [u8; 4],
    fmt_chunk_size: u32,
    audio_format: u16,
    num_channels: u16,
    sample_rate: u32,
    byte_rate: u32,
    sample_align: u16,
    bit_depth: u16,
    magic_data: [u8; 4],
    audio_size: u32
}

fn align_up(n: u32, to: u32) -> u32 {
    return (n + to - 1) / to * to;
}

const ITEM_ALIGN: u32 = 0x20;

fn extract_pac(pac_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file_contents = fs::read(pac_path)?;

    let mut read_offset = 0;
    let mut item_counter = 0;

    while read_offset < file_contents.len() {
        let (_, item_header, _) = unsafe {
            file_contents[read_offset..read_offset + size_of::<PacItemHeader>()].align_to::<PacItemHeader>()
        };
        if item_header.len() != 1 {
            return Err(Box::from("Failed to read header"));
        }
        
        let item_header = item_header[0];
        let magic = std::str::from_utf8(&item_header.magic)?;
        println!("{}: {}: {:?}", read_offset, magic, item_header);
        
        read_offset += size_of::<PacItemHeader>();
        if magic == "JU" {
            read_offset += 2;
        }
        
        let num_channels = 1;
        let sample_rate = item_header.sample_rate / 2;
        let output_path = format!("sample_{}.wav", item_counter);
        let audio_data = &file_contents[read_offset..read_offset + item_header.size2 as usize];
        let wav_header = WavHeader {
            magic_riff: "RIFF".as_bytes().try_into().unwrap(),
            wave_size: (size_of::<WavHeader>() + audio_data.len() - 8) as u32,
            magic_wave: "WAVE".as_bytes().try_into().unwrap(),
            magic_fmt: "fmt ".as_bytes().try_into().unwrap(),
            fmt_chunk_size: 16,
            audio_format: 1,
            num_channels: num_channels as u16,
            sample_rate: sample_rate,
            byte_rate: sample_rate * item_header.sample_size as u32 * num_channels as u32,
            sample_align: item_header.sample_size * num_channels as u16,
            bit_depth: 16,
            magic_data: "data".as_bytes().try_into().unwrap(),
            audio_size: audio_data.len() as u32
        };

        let mut output_file = std::fs::File::create(output_path)?;
        let wav_header_as_ptr = [wav_header].as_ptr().cast();
        let wav_header_as_bytes = unsafe { std::slice::from_raw_parts(wav_header_as_ptr, size_of::<WavHeader>()) };
        output_file.write_all(wav_header_as_bytes)?;
        output_file.write_all(audio_data)?;
        
        read_offset += item_header.size2 as usize;
        read_offset = align_up(read_offset as u32, ITEM_ALIGN) as usize;
        item_counter += 1;
    }
    
    return Ok(());
}

fn create_pac(wav_paths: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let mut output_file = std::fs::File::create("out.pac")?;
    let mut write_size = 0;
    for wav_path in wav_paths {
        let wav = fs::read(wav_path)?;
        let (_, wav_header, _) = unsafe {
            wav[0..size_of::<WavHeader>()].align_to::<WavHeader>()
        };
        if wav_header.len() != 1 {
            return Err(Box::from("Failed to parse wav header"));
        }
        let wav_header = wav_header[0];

        let pac_header = PacItemHeader {
            unk1: 1,
            unk2: 1,
            unk3: wav_header.sample_rate,
            sample_rate: wav_header.sample_rate * 2,
            sample_size: wav_header.bit_depth / 8,
            bit_depth: wav_header.bit_depth,
            magic: [0, 0, 0, 0],
            size1: 0,
            unk6: 0,
            unk7: 0,
            size2: (wav.len() - size_of::<WavHeader>()) as u32
        };
        
        let pac_header_as_ptr = [pac_header].as_ptr().cast();
        let pac_header_as_bytes = unsafe { std::slice::from_raw_parts(pac_header_as_ptr, size_of::<PacItemHeader>()) };
        output_file.write_all(pac_header_as_bytes)?;
        output_file.write_all(&wav[size_of::<WavHeader>()..])?;
        
        write_size += size_of::<PacItemHeader>();
        write_size += pac_header.size2 as usize;
        let padding_size = align_up(write_size as u32, ITEM_ALIGN) as usize - write_size;
        output_file.write_all(&vec![0; padding_size])?;
    }

    return Ok(());
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program_mode = env::args().nth(1).expect("Not enough arguments");
    let program_mode = program_mode.as_bytes();
    if program_mode.len() < 2 || program_mode[0] as char != '-' {
        panic!("Invalid argument");
    }
    match program_mode[1] as char {
        'x' => {
            let pac_path = env::args().nth(2).expect("Missing path argument");
            return extract_pac(&pac_path);
        },
        'c' => {
            let wav_paths = env::args().skip(2).collect::<Vec<_>>();
            return create_pac(wav_paths);
        },
        _ => return Err(Box::from("Invalid mode argument"))
    }
}
