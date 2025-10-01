import os
import json
import base58
import requests
from nacl.signing import SigningKey
from nacl.encoding import Base64Encoder
import urllib3
# Отключаем предупреждения о самоподписанном сертификате
urllib3.disable_warnings(urllib3.exceptions.InsecureRequestWarning)

# --- Настройки ---
API_BASE_URL = "https://localhost:8081"
CERT_FILE = "cert.pem"  # Путь к сертификату сервера (если используется TLS)
KEYPAIR_FILE = os.path.expanduser("~/.config/solana/id.json")  # Путь к файлу ключа Solana
TEST_ENDPOINT = "/api/transactions"  # Эндпоинт, который будем тестировать (для подписи METHOD:PATH:NONCE)
# --- /Настройки ---

def load_keypair(path):
    with open(path, "r") as f:
        arr = json.load(f)
    # Первые 32 байта - это seed
    return bytes(arr[:32])

def main():
    print("--- Генерация токена для PumpNex API ---")

    # 1. Загрузка ключа
    print("1. Загружаем ключ из", KEYPAIR_FILE)
    try:
        seed_bytes = load_keypair(KEYPAIR_FILE)
        signing_key = SigningKey(seed_bytes)
        verify_key_bytes = signing_key.verify_key.encode()  # 32 байта pubkey
        pubkey_b58 = base58.b58encode(verify_key_bytes).decode()
        print(f"   Pubkey (base58): {pubkey_b58}")
    except Exception as e:
        print(f"   Ошибка загрузки ключа: {e}")
        return

    # 2. Получение nonce
    print(f"2. Запрашиваем nonce у {API_BASE_URL}/api/auth/nonce")
    nonce_url = f"{API_BASE_URL}/api/auth/nonce"
    nonce_payload = {"wallet_address": pubkey_b58}
    try:
        response = requests.post(
            nonce_url,
            json=nonce_payload,
            verify=CERT_FILE, # Укажи путь к сертификату сервера
            timeout=10
        )
        response.raise_for_status()
        nonce = response.json().get("nonce")
        if not nonce:
            print("   Ошибка: nonce не найден в ответе")
            return
        print(f"   Получен nonce: {nonce}")
    except requests.exceptions.RequestException as e:
        print(f"   Ошибка запроса nonce: {e}")
        return
    except json.JSONDecodeError:
        print("   Ошибка: получен неверный JSON при запросе nonce")
        return

    # 3. Подпись сообщения "Login:<nonce>"
    print(f"3. Подписываем сообщение 'Login:{nonce}'")
    message_to_sign = f"Login:{nonce}"
    message_bytes = message_to_sign.encode('utf-8') # Важно: UTF-8
    try:
        signed = signing_key.sign(message_bytes)
        signature_bytes = signed.signature # 64 байта
        print(f"   Подпись (hex): {signature_bytes.hex()}")
    except Exception as e:
        print(f"   Ошибка подписи: {e}")
        return

    # 4. Формирование токена Bearer (pubkey + signature -> base58)
    print("4. Формируем токен Bearer")
    token_bytes = verify_key_bytes + signature_bytes
    token = base58.b58encode(token_bytes).decode()
    print(f"   Токен (base58): {token}")

    # 5. Тестирование токена
    print(f"5. Тестируем токен на {API_BASE_URL}{TEST_ENDPOINT}")
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json"
    }
    try:
        response = requests.get(
            f"{API_BASE_URL}{TEST_ENDPOINT}",
            headers=headers,
            verify=CERT_FILE, # Укажи путь к сертификату сервера
            timeout=10
        )
        print(f"   HTTP Status: {response.status_code}")
        print(f"   Ответ: {response.text[:200]}...") # Показываем начало ответа
        if response.status_code == 200:
            print("   ✅ УСПЕШНО! Токен принят.")
        else:
            print("   ❌ ОШИБКА! Токен не принят.")
    except requests.exceptions.RequestException as e:
        print(f"   Ошибка запроса теста: {e}")

if __name__ == "__main__":
    main()
