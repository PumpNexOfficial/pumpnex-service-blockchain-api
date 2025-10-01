use std::process::Command;
use bs58;

fn main() {
    // Получим публичный ключ
    let output = Command::new("solana-keygen")
        .arg("pubkey")
        .arg("/root/.config/solana/id.json")
        .output()
        .expect("Failed to execute solana-keygen");
    
    let pubkey = String::from_utf8(output.stdout).unwrap().trim().to_string();
    println!("Pubkey: {}", pubkey);
    
    // Декодируем публичный ключ из base58
    let pubkey_bytes = bs58::decode(&pubkey).into_vec().unwrap();
    println!("Pubkey bytes ({}): {:?}", pubkey_bytes.len(), pubkey_bytes);
    
    // Подпишем сообщение
    let nonce = "test123";
    let message = format!("Login:{}", nonce);
    println!("Message: {}", message);
    
    let output = Command::new("solana")
        .arg("sign-offchain-message")
        .arg("-k")
        .arg("/root/.config/solana/id.json")
        .arg(&message)
        .output()
        .expect("Failed to execute solana sign");
    
    let signature = String::from_utf8(output.stdout).unwrap().trim().to_string();
    println!("Signature: {}", signature);
    
    // Декодируем подпись
    let sig_bytes = bs58::decode(&signature).into_vec().unwrap();
    println!("Signature bytes ({}): {:?}", sig_bytes.len(), sig_bytes);
    
    // Проверим формат сообщения CLI
    let mut cli_msg = Vec::new();
    cli_msg.extend_from_slice(b"Solana Signed Message:\n");
    cli_msg.extend_from_slice(message.len().to_string().as_bytes());
    cli_msg.push(b'\n');
    cli_msg.extend_from_slice(message.as_bytes());
    
    println!("CLI message format: {}", String::from_utf8_lossy(&cli_msg));
    println!("CLI message hex: {}", hex::encode(&cli_msg));
}
