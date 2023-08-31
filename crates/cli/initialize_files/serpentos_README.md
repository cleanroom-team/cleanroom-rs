# Welcome to Cleanroom based on SerpentOS

I hope you enjoy the experience.

## Getting started

1. Find an environment you can bootstrap Serpent OS from. You can create this
   as a directory containing a SerpentOS installation or by using some
   SerpentOS image file.

2. Check `.envrc` in this directory

   Make sure the variables are sensible:

- `CLRM_ARTIFACTS_DIRECTORY` needs to be the absolute path to some directory
  where the finished artifacts will be stored. This needs several GB of
  space!
- `CLRM_BOOTSTRAP_DIR` is the directory you created above -- leave commented
  if you want to use an image to bootstrap from.
- `CLRM_BOOTSTRAP_IMAGE` is the image file you want to bootstrap from --
  leave commented if you want to use a bootstrap directory
- `CLRM_BUSYBOX` the busybox binary installed on your host system. Install
  or build this as a static binary!
- `CLRM_WORK_DIR` holds temporary data. Make sure you have enough space there

3. Source `.envrc` to make sure the environment variables are set in your shell

4. run `/full/path/to/cleanroom run example_system` to build the system defined
   in the `example_system.toml` file
