const remixImage = new docker.Image("remix", {
    imageName: pulumi.interpolate`${registry.loginServer}/remix:v1`,
    build: { 
      context: "../frontend",
       test: `gloup`
    },
    registry: {
        server: registry.loginServer,
        username:     adminUsername,
        password: adminPassword,
    },
});

const service1Image = new docker.Image("service1", {
    imageName: pulumi.interpolate`${registry.loginServer}/service1:v1`,
    build: { 
        context: `../services/service1` 
    },
    registry: {
        server: registry.loginServer,
        username: adminUsername,
        password: adminPassword,
    },
});

const service2Image = new docker.Image("service2", {
    imageName: pulumi.interpolate`${registry.loginServer}/service2:v1`,
    build: { 
        context: `../services/service2` 
    },
    registry: {
        server : registry.loginServer,
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
     },
    template: {
        containers: [{
            name: "remix",
            image: "node-12",
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
            name: "service1",
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
        ingress: {
            targetPort: 80,
            external: true
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