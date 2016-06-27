#!/bin/sh
git clone https://github.com/eclipse/paho.mqtt.c.git
cd paho.mqtt.c
git checkout develop
make
sudo make install
