use std::{time::Duration, sync::Arc, io::{Read, Seek, BufReader}, path::PathBuf, fs::File};

use rodio::{Source, Decoder};


#[derive(Clone)]
pub struct HordeSourceRaw {
    frames:Vec<HordeWaveFrame>,
    samples:Vec<i16>,
    duration:Option<Duration>,
    name:String,
    path:PathBuf
}
#[derive(Clone)]
pub struct HordeWaveFrame {
    channels:u16,
    sample_rate:u32,
    frame_len:Option<usize>
}

impl HordeSourceRaw {
    pub fn from_file(path:PathBuf) -> Result<Self, ()> {
        match File::open(path.clone()) {
            Ok(file) => {
                Ok(Self::from_decoder(Decoder::new(BufReader::new(file)).unwrap(), path.file_name().unwrap().to_str().unwrap().to_string(), path.clone()))
            },
            Err(_) => Err(())
        } 
    }

    pub fn from_decoder<R:Read + Seek>(mut decoder:Decoder<R>, name:String, path:PathBuf) -> Self {
        let mut samples = Vec::new();
        let mut frames = Vec::new();
        let mut channels = decoder.channels();
        let mut sample_rate = decoder.sample_rate();
        let mut finalised = false;
        while let Some(ticks) = decoder.current_frame_len() {
            frames.push(HordeWaveFrame {
                channels,
                sample_rate,
                frame_len:Some(ticks)
            });
            for i in 0..ticks {
                match decoder.next() {
                    Some(val) => samples.push(val),
                    None => finalised = true,
                }
            }
            channels = decoder.channels();
            sample_rate = decoder.sample_rate();
            if finalised {
                break;
            }
        }
        if !finalised {
            frames.push(HordeWaveFrame { channels, sample_rate, frame_len: None });
        }
        Self {
            frames,
            duration:decoder.total_duration(),
            samples,
            name,
            path
        }
    }
    
}

pub struct HordeSourceData {
    arc:Arc<HordeSourceRaw>
}

impl HordeSourceData {
    pub fn get_iter(&self) -> HordeSource {
        HordeSource { data: self.arc.clone(), index:0, current_frame:0, frame_ticks_left: self.arc.frames[0].frame_len }
    }
    pub fn new(raw:HordeSourceRaw) -> Self {
        Self { arc: Arc::new(raw) }
    }
    pub fn get_name(&self) -> String {
        self.arc.name.clone()
    }
} 

pub struct HordeSource {
    data:Arc<HordeSourceRaw>,
    index:usize,
    current_frame:usize,
    frame_ticks_left:Option<usize>,
}

impl Iterator for HordeSource {
    type Item = i16;
    fn next(&mut self) -> Option<Self::Item> {
        if self.index < self.data.samples.len() {
            self.index += 1;
            match &mut self.frame_ticks_left {
                Some(ticks) => {
                    if *ticks == 0 {
                        self.current_frame += 1;
                        if self.data.frames.len() == self.current_frame {
                            return None
                        }
                        self.frame_ticks_left = self.data.frames[self.current_frame].frame_len;
                    }
                    else {
                        *ticks -= 1;
                    }
                },
                None => ()
            }
            Some(self.data.samples[self.index - 1])
        }
        else {
            None
        }
    }
}

impl Source for HordeSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        self.data.frames[self.current_frame].channels
    }
    fn sample_rate(&self) -> u32 {
        self.data.frames[self.current_frame].sample_rate
    }
    fn total_duration(&self) -> Option<Duration> {
        self.data.duration
    }
}