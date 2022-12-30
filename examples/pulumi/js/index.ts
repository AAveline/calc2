import * as pulumi from "@pulumi/pulumi";
import * as docker from "@pulumi/docker";

import * as app from "@pulumi/azure-native/app";

import {
    resourceGroup,
    workspace,
    workspaceSharedKeys,
    managedEnv,
    registry,
    credentials,
    adminPassword,
    adminUsername
} from "./default";

const remixImage = new docker.Image("remix", {
    imageName: pulumi.interpolate`${registry.loginServer}/remix:v1`,
    build: { context: `../frontend` },
    registry: {
        server: registry.loginServer,
        username: adminUsername,
        password: adminPassword,
    },
});

const service1Image = new docker.Image("service1", {
    imageName: pulumi.interpolate`${registry.loginServer}/service1:v1`,
    build: { context: `../services/service1` },
    registry: {
        server: registry.loginServer,
        username: adminUsername,
        password: adminPassword,
    },
});

const service2Image = new docker.Image("service2", {
    imageName: pulumi.interpolate`${registry.loginServer}/service2:v1`,
    build: { context: `../services/service2` },
    registry: {
        server: registry.loginServer,
        username: adminUsername,
        password: adminPassword,
    },
});

const frontendApp = new app.ContainerApp("frontend", {
    resourceGroupName: resourceGroup.name,
    managedEnvironmentId: managedEnv.id,
    configuration: {
        dapr: {
            enabled: true,
            appPort: 8000,
            appId: "remix"
        },
        ingress: {
            external: true,
            targetPort: 8000,
        },
        registries: [{
            server: registry.loginServer,
            username: adminUsername,
            passwordSecretRef: "pwd",
        }],
        secrets: [{
            name: "pwd",
            value: adminPassword,
        }],
    },
    template: {
        containers: [{
            name: "remix",
            image: remixImage.imageName,
        }],
    },
});

const service1 = new app.ContainerApp("service1", {
    resourceGroupName: resourceGroup.name,
    managedEnvironmentId: managedEnv.id,
    configuration: {
        dapr: {
            appPort: 3000,
            appProtocol: "http",
            enabled: true,
            appId: "service1"
        },
        registries: [{
            server: registry.loginServer,
            username: adminUsername,
            passwordSecretRef: "pwd",
        }],
        secrets: [{
            name: "pwd",
            value: adminPassword,
        }],
    },
    template: {
        containers: [{
            name: "service",
            image: service1Image.imageName,
        }],
    },
});

const service2 = new app.ContainerApp("service2", {
    resourceGroupName: resourceGroup.name,
    managedEnvironmentId: managedEnv.id,
    configuration: {
        dapr: {
            appPort: 3001,
            appProtocol: "http",
            enabled: true,
            appId: "service2"
        },
        registries: [{
            server: registry.loginServer,
            username: adminUsername,
            passwordSecretRef: "pwd",
        }],
        secrets: [{
            name: "pwd",
            value: adminPassword,
        }],
    },
    template: {
        containers: [{
            name: "service2",
            image: service2Image.imageName,
        }],
    },
});

export const url = pulumi.interpolate`https://${frontendApp.configuration.apply((c: any) => c?.ingress?.fqdn)}`;