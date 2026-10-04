//! temperatures from LibreHardwareMonitor (windows).
//! windows does not provide CPU temperature to normal programs; LibreHardwareMonitor has its own driver and can read it
#![cfg_attr(not(windows), allow(dead_code))]
use super::{Provider, Snapshot};
use std::{
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

type Temps = (Option<f32>, Option<f32>); // (cpu, gpu)

pub struct LhmProvider {
    data: Arc<Mutex<Temps>>,
}

impl LhmProvider {
    pub fn new(port: u16) -> Self {
        let data: Arc<Mutex<Temps>> = Arc::default();
        let d = data.clone();
        thread::spawn(move || {
            let mut last = String::new();
            loop {
                let body = fetch(port);
                let t = body.as_deref().map(parse).unwrap_or((None, None));
                let status = match &body {
                    None => format!("LHM: brak połączenia z 127.0.0.1:{port} (włącz Options -> Remote Web Server -> Run)"),
                    Some(b) if t.0.is_none() => {
                        let head: String = b.chars().take(400).collect();
                        format!("LHM: połączono ({} B), ale nie znaleziono temperatury CPU. Początek odpowiedzi: {head}", b.len())
                    }
                    Some(_) => format!("LHM: OK, cpu={:?} gpu={:?}", t.0.map(|v| v.round()), t.1.map(|v| v.round())),
                };
                let key: String = status.chars().take(60).collect();
                if key != last {
                    super::fps::log(&status);
                    last = key;
                }
                *d.lock().unwrap() = t;
                thread::sleep(Duration::from_secs(2));
            }
        });
        Self { data }
    }
}

impl Provider for LhmProvider {
    fn sample(&mut self, out: &mut Snapshot) {
        let (cpu, gpu) = *self.data.lock().unwrap();
        out.cpu_temp = out.cpu_temp.or(cpu);
        out.gpu_temp = out.gpu_temp.or(gpu);
    }
}

fn fetch(port: u16) -> Option<String> {
    let addr: SocketAddr = ([127, 0, 0, 1], port).into();
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_millis(200)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(1))).ok()?;
    s.write_all(b"GET /data.json HTTP/1.0\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .ok()?;
    let mut buf = Vec::new();
    s.read_to_end(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// parse LibreHardwareMonitor json response and return (cpu_temp, gpu_temp)
pub fn parse(json: &str) -> Temps {
    let json = json.replace("\": \"", "\":\"");
    let (mut cpu, mut cpu_rank, mut gpu) = (None, 0u8, None);
    for chunk in json.split("\"Text\":\"").skip(1) {
        let Some(name) = chunk.split('"').next() else {
            continue;
        };
        let Some(vpos) = chunk.find("\"Value\":\"") else {
            continue;
        };
        let val = chunk[vpos + 9..].split('"').next().unwrap_or("").trim();
        if !val.ends_with('C') {
            continue;
        }
        let num: String = val
            .chars()
            .take_while(|c| c.is_ascii_digit() || *c == ',' || *c == '.')
            .map(|c| if c == ',' { '.' } else { c })
            .collect();
        let Ok(t) = num.parse::<f32>() else { continue };
        let n = name.to_lowercase();
        let rank = if n.contains("package") || n.contains("tctl") || n.contains("tdie") {
            3
        } else if n.contains("core max") || n == "cpu" {
            2
        } else if n.starts_with("core") || n.starts_with("cpu") || n.starts_with("ccd") {
            1
        } else {
            0
        };
        if rank > cpu_rank || (rank == 1 && cpu_rank == 1 && cpu.map_or(true, |c| t > c)) {
            cpu = Some(t);
            cpu_rank = rank;
        }
        if gpu.is_none() && n.starts_with("gpu core") {
            gpu = Some(t);
        }
    }
    (cpu, gpu)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_sample() {
        let j = r#"{"Text":"PC","Children":[{"Text":"AMD Ryzen","Value":"","Children":[
          {"Text":"CPU Package","Min":"1,0 W","Value":"45,3 W","Max":"x"},
          {"Text":"Core #1","Value":"50,0 °C"},
          {"Text":"Core (Tctl/Tdie)","Min":"30","Value":"61,5 °C","Max":"80"},
          {"Text":"CPU Total","Value":"12,0 %"},
          {"Text":"GPU Core","Value":"48,0 °C"},{"Text":"GPU Core","Value":"30,0 %"}]}]}"#;
        assert_eq!(super::parse(j), (Some(61.5), Some(48.0)));
    }

    #[test]
    fn parses_spaced_format_and_core_fallback() {
        let j = r#"{"id": 0, "Text": "Sensor", "Children": [{"id": 3, "Text": "Intel Core i5", "Min": "", "Value": "", "Children": [
          {"id": 4, "Text": "Core #1", "Min": "30,0 °C", "Value": "52,0 °C", "Max": "70,0 °C", "Type": "Temperature"},
          {"id": 5, "Text": "Core #2", "Min": "30,0 °C", "Value": "58,0 °C", "Max": "70,0 °C", "Type": "Temperature"}]}]}"#;
        assert_eq!(super::parse(j), (Some(58.0), None));
    }
}
