group "default" {
  targets = ["build"]
}

target "build" {
  dockerfile = "Dockerfile"
  platforms = [
    "linux/amd64",
    // "linux/arm/v6",
    // "linux/arm/v7",
    "linux/arm64",
    // "linux/386"
  ]
}
