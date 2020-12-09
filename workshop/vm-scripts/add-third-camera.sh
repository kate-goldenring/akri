#!/bin/bash
sudo v4l2loopback-ctl add -n "spokes" /dev/video7
sudo gst-launch-1.0 -v videotestsrc pattern=spokes ! "video/x-raw,width=640,height=480,framerate=10/1" ! avenc_mjpeg ! v4l2sink device=/dev/video7 > camera-logs/spokes.log 2>&1 &