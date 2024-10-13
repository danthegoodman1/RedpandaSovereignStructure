set -e # exit on error

echo "Provide new lines via stdin to produce to the input topic..."
docker compose exec -T redpanda-0 rpk topic produce input --brokers localhost:9092
