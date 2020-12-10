# Using the v4l2loopback Kernel Module
If you would like to enable [dynamic device management with the v4l2loopmake module](https://github.com/umlaeute/v4l2loopback#dynamic-device-management), you must clone and build the module, as the functionality has not been added to a release yet. This will walk through those steps. Otherwise, you can follow the steps outlines in the [end-to-end demo](../docs/end-to-end-demo.md#set-up-mock-udev-video-devices).
1. Clone the repo
  ```sh
  git clone https://github.com/umlaeute/v4l2loopback.git
  ```
1. Build the module. Build `v4l2loopback-ctl` via `sudo make install-utils` to enable dynamic device management.
  ```sh
  make && sudo make install
  sudo make install-utils
  sudo depmod -a
  ```
1. Create two fake devices
  ```sh
  sudo modprobe v4l2loopback exclusive_caps=1 video_nr=1,2
  ```