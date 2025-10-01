use ed25519_dalek::{PublicKey, Signature, Verifier};
use bs58;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pubkey_str = "4iGDwygceA9cfAfy9wZnCm42mxTs8mZ9W3CqknU3WVvB";
    let signature_str = "34QdKSY8g2zkvni8WQRa983iakGq8QDV8xNzcpfKFh5xqeFvvD3DEgU8JJBjbqdasgNAXAwzecwTWHYsa8MGAejr";
    let message = "Login:test123";

    // Декодируем публичный ключ
    let pubkey_bytes = bs58::decode(&pubkey_str).into_vec()?;
    let pubkey = PublicKey::from_bytes(&pubkey_bytes)?;

    // Декодируем подпись
    let sig_bytes = bs58::decode(&signature_str).into_vec()?;
    let signature = Signature::from_bytes(&sig_bytes)?;

    // 1. Проверим простое сообщение
    println!("Testing raw message: '{}'", message);
    let raw_ok = pubkey.verify(message.as_bytes(), &signature).is_ok();
    println!("Raw verification: {}", raw_ok);

    // 2. CLI формат
    let mut cli_msg = Vec::new();
    cli_msg.extend_from_slice(b"Solana Signed Message:\n");
    cli_msg.extend_from_slice(message.len().to_string().as_bytes());
    cli_msg.push(b'\n');
    cli_msg.extend_from_slice(message.as_bytes());
    
    println!("Testing CLI format: '{}'", String::from_utf8_lossy(&cli_msg));
    let cli_ok = pubkey.verify(&cli_msg, &signature).is_ok();
    println!("CLI format verification: {}", cli_ok);

    Ok(())
}
