#!/bin/bash

# Ask gst-launch nicely to terminate.
if pgrep gst-launch-1.0 > /dev/null; then
    sudo pkill gst-launch-1.0
fi

# Forcibly terminate.
if pgrep gst-launch-1.0 > /dev/null; then
    sudo pkill -9 gst-launch-1.0
fi

# Launch again.
sudo gst-launch-1.0 -v videotestsrc pattern=ball ! "video/x-raw,width=640,height=480,framerate=10/1" ! avenc_mjpeg ! v4l2sink device=/dev/video1 > camera-logs/ball.log 2>&1 &
sudo gst-launch-1.0 -v videotestsrc pattern=smpte horizontal-speed=1 ! "video/x-raw,width=640,height=480,framerate=10/1" ! avenc_mjpeg ! v4l2sink device=/dev/video2 > camera-logs/smpte.log 2>&1 &