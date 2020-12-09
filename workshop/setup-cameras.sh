# Install kernel module to mock video cameras and start two instances
echo "Install v4l2loopback kernel module and dependencies"
git clone https://github.com/umlaeute/v4l2loopback.git
cd v4l2loopback
make && sudo make install
sudo make install-utils
sudo depmod -a
sudo modprobe v4l2loopback exclusive_caps=1 video_nr=1,2

sudo apt-get install -y \
  libgstreamer1.0-0 gstreamer1.0-tools gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good gstreamer1.0-libav

echo "Start two mock cameras at /dev/video1 and /dev/video2"
sudo gst-launch-1.0 -v videotestsrc pattern=ball ! "video/x-raw,width=640,height=480,framerate=10/1" ! avenc_mjpeg ! v4l2sink device=/dev/video1 &

sudo gst-launch-1.0 -v videotestsrc pattern=smpte horizontal-speed=1 ! "video/x-raw,width=640,height=480,framerate=10/1" ! avenc_mjpeg ! v4l2sink device=/dev/video2 &

echo "Installation script complete"