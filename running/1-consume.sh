set -e # exit on error

echo "Consuming from 'structured' topic..."
docker compose exec redpanda-0 rpk topic consume structured --brokers localhost:9092 --format json
