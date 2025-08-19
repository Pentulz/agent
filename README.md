# Pentulz Agent

<a name="readme-top"></a>

<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
        <a href="#built-with">Built With</a>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#development">Development</a></li>
        <li><a href="#docker">Docker</a></li>
        <li><a href="#examples">Examples</a></li>
      </ul>
    </li>
    <li><a href="#license">License</a></li>
    <li><a href="#contacts">Contacts</a></li>
  </ol>
</details>

### Built With

- [Rust 1.89][rust]

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->

## Getting Started

This is a daemon that will retrieve from a remote HTTP server which tasks should be executed on the deployed machine and send back the results of those tasks.

This application cannot be used on its own. It is simply a slave that will retrieve which tasks should be executed on the master server.

### Prerequisites

#### Rust 1.89

- [rustup][rustup]

  First, install the rustup tool. Then install the correct rust's version:
  ```sh
  rustup toolchain install stable # (1.89 at the time of this writing)
  
  rustup  default stable
  ```

#### Development

Use the cargo to install dependencies, build and package the project.

```sh
# install the dependencies and build the project
cargo build
# run
cargo run
```

#### Docker

The application can be used with docker.

```sh
# TODO
```

Or you can use docker compose

```sh
# TODO
```

#### Examples

```sh
# TODO
```

#### Github actions

If you want to test the `Github actions` on your machine, you can use [act](https://github.com/nektos/act).

Launch the publish workflow with the following command:

```sh
act --artifact-server-path /tmp/artifacts
```

This will upload the build artifact (binary file) into that directory.

<!-- CONTRIBUTING -->

## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->

## License

Distributed under the MIT License. See `LICENSE` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTACT -->

## Contacts

- [Mondotosz](https://github.com/Mondotosz)
- [NATSIIRT](https://github.com/NATSIIRT)
- [Thynkon](https://github.com/Thynkon)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->

[rust]: https://www.rust-lang.org
[docker]: https://www.docker.com
[rustup]: https://rustup.rs
