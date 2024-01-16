# Secret / Certificat Azure Exporter

Exports the status of Azure secrets and certificates as Prometheus metrics

## Creation of Azure Active Directory app

In Azure Active Directory, create an app with the following settings :
- Supported account types: Accounts in this organizational directory only
- Authorized APIs: Microsoft Graph > Application.Read.All
- Certificates & secrets: create an "APPLICATION-EXPORTER" client secret and note its value

Once the app created, note the **Microsoft Tenant ID** and the **App ID (client)**

## Getting Started

Provide the following environment variables (locally you can use a `.env` file) :

```
TENANT_ID=<Microsoft Tenant ID>
CLIENT_ID=<App ID (client)>
CLIENT_SECRET=<"APPLICATION-EXPORTER" client secret>
SCOPE=https://graph.microsoft.com/.default
```

Run the following commands :

if you have cargo already installed:
```
cargo build --release
```

if you want to build a OCI image:
```
docker build . -t secret-certificat-azure-exporter
```