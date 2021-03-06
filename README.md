[![Contributors][contributors-shield]][contributors-url]
[![Issues][issues-shield]][issues-url]



<br />
<p align="center">
  <a href="https://github.com/cachance7/fuzzy5e">
    <img src="logo.png" alt="Logo" width="160">
  </a>

  <h3 align="center">Fuzzy5e</h3>

  <p align="center">
    A 5th edition SRD5 reference for your terminal.
  </p>
</p>

<p align="center">
<img src="fuzzy5e.gif" height="400" width="600"/>
</p>


<!-- TABLE OF CONTENTS -->
## Table of Contents

* [About the Project](#about-the-project)
  * [Built With](#built-with)
* [Getting Started](#getting-started)
  * [Prerequisites](#prerequisites)
  * [Installation](#installation)
  * [Usage](#usage)
* [Contributing](#contributing)
* [License](#license)
* [Contact](#contact)
* [Acknowledgements](#acknowledgements)



<!-- ABOUT THE PROJECT -->
## About The Project

As a DM I find myself constantly looking up spells and monster stat blocks while running the game. As a programmer, the terminal is my most concise at-a-glance space for text information. After years of struggling to juggle browser windows and tabs, I decided to build this as a sort of quick reference and HUD for my most commonly referenced information.

NOTE: This is very much a **_work in progress_**.

<img width="1440" alt="Screen Shot 2020-05-05 at 11 29 01 AM" src="https://user-images.githubusercontent.com/1068829/81101928-aec2f280-8ec3-11ea-8138-387630ecd843.png">

_Tiling in tmux turns out to be exactly what I wanted._



### Built With

* [Rust](https://www.rust-lang.org/) - this app is written in rust
* [tuikit](https://github.com/lotabout/tuikit) - leaned on this heavily for the terminal UI presentation
* [5e-database](https://github.com/bagelbits/5e-database) - SRD5 data is sourced from here


<!-- GETTING STARTED -->
## Getting Started

fuzzy5e is a terminal UI written in Rust. It has been tested on Mac OSX Catalina and Linux. YMMV on Windows.

### Prerequisites

[Rust >=1.40](https://www.rust-lang.org/tools/install) (if building from source)

### Installation (from release)

1. Download [latest binary and index](https://github.com/cachance7/fuzzy5e/releases/latest)
2. Extract locally
3. Run the executable and point to index

```sh
fuzzy5e -i path/to/indexdir
```

### Installation (from git repo)

1. Clone the repo
```sh
git clone https://github.com/cachance7/fuzzy5e
```
2. Build and run the executable
```sh
cd fuzzy5e && cargo run
```
3. Enjoy!


### Usage

- Start the program with `fuzzy5e` helper script
- Type to begin searching
- `Ctrl+N` / `Ctrl+P`: select next / previous match
- `Up` / `Down` / `PgUp` / `PgDown`: scroll the selected content up or down
- `Enter`: show the selected match full window
- `Esc`: quit

<!-- CONTRIBUTING -->
## Contributing

If you'd like to contribute, feel free to open up a PR and let's make it work!



<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE` for more information.

All searchable content is derived from SRD5 and is used courtesy of the [OGL-1.0a](https://github.com/cachance7/fuzzy5e/blob/master/OGL-1.0a.txt)


<!-- CONTACT -->
## Contact

[Casey Chance](mailto:casey@chance.email?subject=fuzzy5e)

Project Link: [https://github.com/cachance7/fuzzy5e](https://github.com/cachance7/fuzzy5e)


## Acknowledgements
* [dnd5api](http://dnd5eapi.co/)
* [Gary Gygax](https://en.wikipedia.org/wiki/Gary_Gygax)
* Logo built using icons made by Flat Icons from www.flaticon.com


<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/cachance7/fuzzy5e.svg?style=flat-square
[contributors-url]: https://github.com/cachance7/fuzzy5e/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/cachance7/fuzzy5e.svg?style=flat-square
[forks-url]: https://github.com/cachance7/fuzzy5e/network/members
[stars-shield]: https://img.shields.io/github/stars/cachance7/fuzzy5e.svg?style=flat-square
[stars-url]: https://github.com/cachance7/fuzzy5e/stargazers
[issues-shield]: https://img.shields.io/github/issues/cachance7/fuzzy5e.svg?style=flat-square
[issues-url]: https://github.com/cachance7/fuzzy5e/issues
[linkedin-shield]: https://img.shields.io/badge/-LinkedIn-black.svg?style=flat-square&logo=linkedin&colorB=555
[linkedin-url]: https://linkedin.com/in/casey-chance-9ba0b6a
[product-screenshot]: images/screenshot.png
