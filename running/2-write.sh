set -e # exit on error

echo "Writing to structured topic..."
docker compose exec redpanda-0 rpk topic produce input --brokers localhost:9092
