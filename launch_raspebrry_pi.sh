#!/bin/bash
echo "Waiting for network connectivity"
while ! ping -c 1 8.8.8.8 &> /dev/null;do
    sleep 2
    echo "Internet not available, retry in 2s"
done

echo "Network connectivity available"
./home/guyot/Documents/release/raspberry_pi

