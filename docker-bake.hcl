variable "TAG" {
    default = "latest"
}

variable "REGISTRY" {
    default = "localhost"
}

group "default" {
    targets = ["image"]
}

target "image" {
    dockerfile = "Dockerfile"
    tags = ["${REGISTRY}/actix-web-starter:${TAG}"]
    platforms = ["linux/amd64"]
}

target "image-all" {
    inherits = ["image"]
    platforms = ["linux/amd64", "linux/arm64", "linux/arm/v6", "linux/arm/v7"]
}