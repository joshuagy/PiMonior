#!/bin/bash

# Starting/pulling Grafana
if [[ $(sudo docker ps -a) == */grafana* ]]; then
  sudo docker container start grafana
  echo "Starting Grafana."
else
  echo "Creating Docker image for Graphana."
  sudo docker run -d -p 3000:3000 --name=grafana \
    -e "GF_PLUGINS_PREINSTALL=custom-plugin@@http://plugin-domain.com/my-custom-plugin.zip,grafana-clock-panel" \
    grafana/grafana-enterprise
fi

# Starting/pulling Prometheus
if [[ $(sudo docker ps -a) == */prometheus* ]]; then
  sudo docker container start prometheus
  echo "Starting Prometheus."
else
  echo "Creating Docker image and container for Prometheus."
  sudo docker volume create prometheus-data
  sudo docker run -d \
    --name prometheus \
    -p 9090:9090 \
    -v ./prometheus.yml:/etc/prometheus/prometheus.yml \
    -v prometheus-data:/prometheus \
    prom/prometheus
fi

# Starting/pulling Pushgateway
if [[ $(sudo docker ps -a) == */pushgateway* ]]; then
  sudo docker container start pushgateway
  echo "Starting Pushgateway."
else
  echo "Creating Docker image and container for Pushgateway."
  sudo docker run -d --name pushgateway -p 9091:9091 prom/pushgateway
fi