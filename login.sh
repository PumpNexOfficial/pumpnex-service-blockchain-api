#!/bin/bash
set -e

echo "🚀 FINAL ATTEMPT - CORRECT SOLANA SIGNING WITH sign-offchain-message (HEX STRING)"

# Проверка Redis
echo "➤ Checking Redis..."
docker-compose exec -T redis redis-cli ping | grep -q "PONG" && echo "✅ Redis OK" || { echo "❌ Redis failed"; exit 1; }

# Публичный ключ
PUBKEY=$(solana-keygen pubkey ~/.config/solana/id.json)
echo "Wallet: $PUBKEY"

# Получаем nonce
RESPONSE=$(curl -s --resolve localhost:8081:127.0.0.1 \
  https://localhost:8081/api/auth/nonce \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\":\"$PUBKEY\"}" \
  --cacert cert.pem)

NONCE=$(echo "$RESPONSE" | grep -oP '"nonce":"[^"]+"' | cut -d'"' -f4)
echo "Nonce: $NONCE"

# === ФОРМИРУЕМ СТАНДАРТНОЕ SOLANA СООБЩЕНИЕ ===
MESSAGE="Login:$NONCE"
SOLANA_MESSAGE=$'\x18Solana Signed Message:\n'$(echo -n "$MESSAGE" | wc -c)"$MESSAGE"

# === ПЕРЕВОДИМ В HEX ===
SOLANA_MESSAGE_HEX=$(echo -n "$SOLANA_MESSAGE" | xxd -p | tr -d '\n')
echo "Solana Standard Message (hex): $SOLANA_MESSAGE_HEX"

# Подписываем сообщение с помощью `solana sign-offchain-message`
echo "Signing with solana CLI..."
SIG_OUTPUT=$(solana sign-offchain-message "$SOLANA_MESSAGE_HEX" --keypair ~/.config/solana/id.json)
SIG=$(echo "$SIG_OUTPUT" | grep -oP 'Signature: \K.*')

echo "Signature: $SIG"

# Отправляем запрос
echo "Sending login request..."
LOGIN_RESPONSE=$(curl -s --resolve localhost:8081:127.0.0.1 \
  https://localhost:8081/api/auth/login \
  -H "Content-Type: application/json" \
  -d "{\"wallet_address\":\"$PUBKEY\",\"signature\":\"$SIG\"}" \
  --cacert cert.pem)

echo "Login response: $LOGIN_RESPONSE"

if echo "$LOGIN_RESPONSE" | grep -q "access_token"; then
    echo "🎉 FINALLY SUCCESS!"
    ACCESS_TOKEN=$(echo "$LOGIN_RESPONSE" | grep -oP '"access_token":"[^"]+"' | cut -d'"' -f4)
    echo "Access Token: $ACCESS_TOKEN"
else
    echo "💥 COMPLETE FAILURE"
    echo "Check server logs above for hex values"
fi
