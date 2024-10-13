set -e # exit on error

echo "Writing to input topic..."
cat example_email.txt | docker compose exec -T redpanda-0 rpk topic produce input --brokers localhost:9092
