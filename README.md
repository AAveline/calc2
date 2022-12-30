# CappS (ContainerApps Serializer)

## What is it ?
This should be an unified `Azure ContainerApps` serializer from multi providers such as `Pulumi`, `Azure ARM` and `Terraform` to `Docker compose`. The goal is to serialize IAC Cloud configuration to a local compose emulation.

## How to do this ?
This serializer should handle some IAC languages such as Bicep, Yaml, or language used in CDK, parse and convert them to an unified format who could be deserialized to compose.
At this moment, only the `Pulumi` provider with `Yaml` and `Javascript` languages are supported. In the futur, the `Json` format from `Azure` provider will be handled.

## How it works ?
- Get the binary from github release
- Go to the folder where you run your IAC provider (Pulumi for the moment) and run the binary `./<binary> pulumi --input <file>.yml`

## Limitations
- Cannot handle multiple files as input for now