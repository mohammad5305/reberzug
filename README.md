<p align="center">
  <img align="center" src="https://github.com/mohammad5305/reberzug/assets/71145952/be6db9a5-3b85-431d-8d75-791fcc9dd516" width="25%">
  <h2 align="center">User-friendly ueberzug alternative</h2>
</p>

reberzug is an easy to use ueberzug alternative with simple configuration that is blazingly fast for showing images in terminal using child windows.

Currently only X11 is supported

![demo](https://i.imgur.com/6e92Ld5.gif)
## Installation
### pre-built binaries
Download from `release` section of repo

### Building from source
install rust toolchain and run
```sh
git clone https://github.com/mohammad5305/reberzug && cd reberzug
cargo build --release

```
The binary file will be in `./target/release/reberzug`
```
cp ./target/release/reberzug ~/.local/bin
chmod +x ~/.local/bin
```

## Usage
Just give width, height arguments followed by image path 
```sh
reberzug -W 300 -H 500 /tmp/image.png
```
for downscaling/upscaling use box/nearest resize algorithms respectively
```sh
reberzug -W 300 -H 500 -r box /tmp/image.png
```

For more information run `reberzug -h`

### Fzf preview
for better postioning needs `xdotool` to get width, height of terminal.
```sh
IFS="x" read -r WIN_WIDTH WIN_HEIGHT <<< "$(xdotool getwindowfocus getwindowgeometry | grep -ioP 'Geometry:\K.+')"

COLS=$(tput cols)
LINES=$(tput lines)

WIDTH_RATIO=$((WIN_WIDTH/COLS))
HEIGHT_RATIO=$((WIN_HEIGHT/LINES))

draw_image (){
    # 60 is supposed to be based on fontsize :)
    X=$((FZF_PREVIEW_COLUMNS * WIDTH_RATIO + 60))
    Y=$((1 * HEIGHT_RATIO))
    /tmp/reberzug -W 500 -H 300 -r box -x $X -y $Y $1
}

export WIDTH_RATIO HEIGHT_RATIO
export -f draw_image

ls -1 <path> |fzf --preview "draw_image <path>/{}"
```
