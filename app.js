// Definition des librairies utilisées 
const HttpsProxyAgent = require('https-proxy-agent');
const express = require('express'); // Exposition de l'API
const client = require('prom-client'); // Création de métrics pour prometheus
require("dotenv").config();

const app = express();

// Récupération du proxy à utiliser
var proxy = process.env.http_proxy || 'http://proxy-http:8080';
console.log("Utilisation du proxy %j", proxy);
const axiosDefaultConfig = {
    proxy: false,
    httpsAgent: new HttpsProxyAgent(proxy)
};
const axios = require ('axios').create(axiosDefaultConfig);

// Création du registre qui stocke les métriques de proxima
let register = new client.Registry();

// Définition du nom de registre où seront stockés les métriques
register.setDefaultLabels({
  app: "Azure Certificat Expiration"
})

// On récupère les métriques par défauts
client.collectDefaultMetrics({ register })

// On récupère le token 
function getToken() {

  const params = new URLSearchParams();
  params.append('client_id', process.env.CLIENT_ID)
  params.append('scope', process.env.SCOPE)
  params.append('client_secret', process.env.CLIENT_SECRET)
  params.append('grant_type', "client_credentials")
  
  let options = {
    method: 'post',
    url: `https://login.microsoftonline.com/${process.env.TENANT_ID}/oauth2/v2.0/token`,
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    data: params
  }

  return axios.request(options);
}

// On récupère les données et on les envois dans ParseValue
function GetSubscriptionList() {
  getToken()
    .then(function (response) {
      let options = {
        method: 'get',
        url: "https://graph.microsoft.com/v1.0/applications",
        headers: {
          'Authorization': 'Bearer ' + response.data.access_token
        }
      }
      axios.request(options)
        .then(function (response) {
          ParseValue(response.data);
        })
        .catch(function (error) {
          console.log(error);
        });
    })
    .catch(function (error) {
      console.log(error);
    });

}

function ParseValue(subscriptionlist) {
  // On nettoie notre registre pour ajouter les données actualisées
  client.register.clear()

  // On boucle sur chaque application
  subscriptionlist.value.forEach(application => {
    var i = 0;
    // On nettoie les noms pour remove les caractères spéciaux + espaces (pour la conversion en métriques)
    application.appId = application.appId.replaceAll('-', "_");
    application.displayName = application.displayName.replaceAll('-', "_");
    application.displayName = application.displayName.replaceAll(' ', "");
    application.displayName = application.displayName.replaceAll('é', "e");
    application.displayName = application.displayName.replaceAll('ê', "e");
    application.displayName = application.displayName.replaceAll('(', "");
    application.displayName = application.displayName.replaceAll(')', "");

    if (application.passwordCredentials.length > 0) {
      application.passwordCredentials.forEach(password => {

        applicationvalue = "application_" + application.appId + "_" + i;

        var ApplicationSecretStatus = new client.Gauge({
          name: applicationvalue,
          help: "Secret N°" + i + " pour l'application " + application.displayName,
          labelNames: ['application', 'type'],
        });

        var date_now = new Date().getTime();
        var date_end = new Date(password.endDateTime).getTime();
        
        // Vérifier si date_now est après date_end
        if (date_now > date_end) {
            console.error("Le Certificat de l'application " + application.displayName + " a expiré le " + password.endDateTime);
            var date_restant = 0;
        } else {
            var date_restant = Math.ceil((date_end - date_now) / (1000 * 3600 * 24));
        }

        ApplicationSecretStatus.set({ application: application.displayName, type: "secret" }, date_restant);
        // Si la métriques existe déjà, on la set pas de nouveau dans le registre
        try { register.registerMetric(ApplicationSecretStatus); } catch { }
        i++
      })
    }

    if (application.keyCredentials.length > 0) {
      application.keyCredentials.forEach(certificat => {
        applicationvalue = "application_" + application.appId + "_" + i;

        var ApplicationCertificatStatus = new client.Gauge({
          name: applicationvalue,
          help: "Certificat N°" + i + " pour l'application " + application.displayName,
          labelNames: ['application', 'type'],
        });

        var date_start = new Date(certificat.startDateTime).getTime();
        var date_end = new Date(certificat.endDateTime).getTime();
        var date_restant = Math.ceil((date_end - date_start) / (1000 * 3600 * 24));
        ApplicationCertificatStatus.set({ application: application.displayName, type: "certificat" }, date_restant);
        // Si la métriques existe déjà, on la set pas de nouveau dans le registre
        try { register.registerMetric(ApplicationCertificatStatus); } catch { }
        i++
      })
    }

  });

}

// On expose nos métriques
app.get('/metrics', async function (req, res) {
  GetSubscriptionList()

  res.setHeader('Content-type', register.contentType);
  res.end(await register.metrics())
})

app.get('/', async function (req, res) {
  res.send("I'm Alive :D");
})


app.listen(3000, () => {
  console.log(`Azure Certificat Export en marche ! :) `);
});


GetSubscriptionList()