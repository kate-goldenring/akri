#!/bin/bash
sudo modprobe -r v4l2loopback
sudo modprobe v4l2loopback exclusive_caps=1 video_nr=1,2