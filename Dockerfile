FROM node:lts
WORKDIR /usr/src/app

ARG NPM_REGISTRY='https://registry.npmjs.org'

COPY package*.json ./
RUN npm install --registry $NPM_REGISTRY
COPY . .

EXPOSE 3000
CMD [ "node", "app.js" ]