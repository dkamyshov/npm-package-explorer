# npm-package-explorer

This software serves files directly from NPM packages. Useful in case of building a CDN or package documentation server.

## Running (with Docker)

The image name is [danilkamyshov/npm-package-explorer](https://hub.docker.com/r/danilkamyshov/npm-package-explorer).

Place the `npm-package-explorer.config.toml` (see `example.npm-package-explorer.config.toml` in the root of the repository) file in `/usr/dist`, like so:

```
sudo docker run \
  -v $(pwd)/example.npm-package-explorer.config.toml:/usr/dist/npm-package-explorer.config.toml \
  -p 8080:8080 \
  -it --rm \
  danilkamyshov/npm-package-explorer
```

Then, navigate to the [index page](http://localhost:8080/) of the explorer. After that, [try](http://localhost:8080/show/react/17.0.0/README.md) [viewing](http://localhost:8080/show/react/17.0.0/umd/react.development.js) [some](http://localhost:8080/show/react/17.0.0/index.js) [files](http://localhost:8080/show/react/17.0.0/build-info.json).
