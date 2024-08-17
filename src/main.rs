use sha2::{Sha256, Digest};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering};
use std::thread;
use std::io::{self, Write};
use ctrlc;
use itoa;

const NUM_THREADS: usize = 16;

#[inline(always)]
fn count_leading_zeros(hash: &[u8]) -> u32 {
    let zeros = hash.iter().take_while(|&&byte| byte == 0).count();
    (zeros * 8) as u32 + hash.get(zeros).map_or(0, |&byte| byte.leading_zeros())
}

fn process_range(start: u64, step: u64, best_bits: &AtomicU32, current: &AtomicU64, running: &AtomicBool) {
    let mut i = start;
    let mut hasher = Sha256::new();
    let mut buffer = itoa::Buffer::new();

    while running.load(Ordering::Relaxed) {
        let number_str = buffer.format(i);
        hasher.update(number_str.as_bytes());
        let hash = hasher.finalize_reset();

        let leading_zeros = count_leading_zeros(&hash);
        let current_best = best_bits.load(Ordering::Relaxed);

        if leading_zeros > current_best && best_bits.compare_exchange(current_best, leading_zeros, Ordering::SeqCst, Ordering::Relaxed).is_ok() {
            println!("Novo melhor número encontrado: {}", i);
            println!("Bits 0 à esquerda: {}", leading_zeros);
            println!("Hash: {:x}", hash);
            println!();
        }

        i += step;
        current.fetch_max(i, Ordering::Relaxed);
    }
}

fn main() -> io::Result<()> {
    print!("Digite o número inicial: ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let start_number: u64 = input.trim().parse().expect("Por favor, digite um número válido");

    let best_bits = AtomicU32::new(0);
    let current = AtomicU64::new(start_number);
    let running = AtomicBool::new(true);

    let best_bits = Arc::new(best_bits);
    let current = Arc::new(current);
    let running = Arc::new(running);

    ctrlc::set_handler({
        let current = Arc::clone(&current);
        let running = Arc::clone(&running);
        move || {
            println!("\nPrograma interrompido.");
            println!("Último número calculado: {}", current.load(Ordering::Relaxed));
            running.store(false, Ordering::Relaxed);
        }
    }).expect("Erro ao configurar o manipulador de Ctrl+C");

    let handles: Vec<_> = (0..NUM_THREADS)
        .map(|t| {
            let best_bits = Arc::clone(&best_bits);
            let current = Arc::clone(&current);
            let running = Arc::clone(&running);
            thread::spawn(move || {
                process_range(start_number + t as u64, NUM_THREADS as u64, &best_bits, &current, &running);
            })
        })
        .collect();

    for handle in handles {
        handle.join().unwrap();
    }

    Ok(())
}