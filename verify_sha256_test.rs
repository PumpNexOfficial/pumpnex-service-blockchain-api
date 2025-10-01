use ed25519_dalek::{Keypair, PublicKey, Signature, Signer, Verifier};
use bs58;
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Создадим тестовый ключ для проверки
    let mut csprng = OsRng;
    let keypair = Keypair::generate(&mut csprng);
    let pubkey = keypair.public;

    // Подпишем и проверим сообщение с SHA256
    let message = b"Hello, world!";
    let digest = Sha256::digest(message);
    let signature = keypair.sign(&digest);
    
    // Проверим с verify_prehashed
    let result = pubkey.verify_prehashed(digest, None, &signature);
    println!("SHA256 signing test: {}", result.is_ok());

    // Теперь проверим с реальными данными из CLI
    let pubkey_str = "4iGDwygceA9cfAfy9wZnCm42mxTs8mZ9W3CqknU3WVvB";
    let signature_str = "34QdKSY8g2zkvni8WQRa983iakGq8QDV8xNzcpfKFh5xqeFvvD3DEgU8JJBjbqdasgNAXAwzecwTWHYsa8MGAejr";
    let message = "Login:test123";

    // Декодируем публичный ключ
    let pubkey_bytes = bs58::decode(&pubkey_str).into_vec()?;
    let pubkey = PublicKey::from_bytes(&pubkey_bytes)?;

    // Декодируем подпись
    let sig_bytes = bs58::decode(&signature_str).into_vec()?;
    let signature = Signature::from_bytes(&sig_bytes)?;

    // Формат CLI
    let mut cli_msg = Vec::new();
    cli_msg.extend_from_slice(b"Solana Signed Message:\n");
    cli_msg.extend_from_slice(message.len().to_string().as_bytes());
    cli_msg.push(b'\n');
    cli_msg.extend_from_slice(message.as_bytes());
    
    println!("CLI message: '{}'", String::from_utf8_lossy(&cli_msg));

    // Проверим с SHA256
    let digest = Sha256::digest(&cli_msg);
    let result = pubkey.verify_prehashed(digest, None, &signature);
    println!("CLI with SHA256 verification: {}", result.is_ok());

    Ok(())
}
