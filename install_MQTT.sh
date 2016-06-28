#!/bin/bash
#
# run as root
 
echo "Adding key."
wget http://repo.mosquitto.org/debian/mosquitto-repo.gpg.key
sudo apt-key add mosquitto-repo.gpg.key

echo "Adding repo to apt."
cd /etc/apt/sources.list.d/
sudo wget http://repo.mosquitto.org/debian/mosquitto-wheezy.list

echo "Updating db"
apt-get update

echo "Installing mosquitto"
apt-get install mosquitto

echo "Starting MQTT service"
sudo service mosquitto start

echo "Subscribung to test/topic for one message"
echo "(in background to prevent blocking the shell)"
mosquitto_sub -v -C 1 -t 'test/topic' &
sleep 1 #short sleep to get prints in right order

echo "Publishing on test/topic"
mosquitto_pub -t 'test/topic' -m 'helloWorld'

echo "Stopping MQTT service"
sudo service mosquitto stop



