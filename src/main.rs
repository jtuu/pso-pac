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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args();
    let argc = args.len();
    let pac_path = args.next_back();
    if argc < 2 || pac_path.is_none() {
        return Err(Box::from("Missing argument"));
    }

    let pac_path = pac_path.unwrap();
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
        let p = [wav_header].as_ptr().cast();
        let d = unsafe { std::slice::from_raw_parts(p, size_of::<WavHeader>()) };
        output_file.write_all(d)?;
        output_file.write_all(audio_data)?;
        
        read_offset += item_header.size2 as usize;
        read_offset = align_up(read_offset as u32, ITEM_ALIGN) as usize;
        item_counter += 1;
    }
    return Ok(());
}
