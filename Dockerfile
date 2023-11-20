FROM node:18.18.2-bookworm-slim
WORKDIR /usr/src/app

ARG NPM_REGISTRY='https://registry.npmjs.org'

COPY package*.json ./
RUN npm ci --omit dev --registry $NPM_REGISTRY && npm cache clean --force
COPY . .

EXPOSE 3000
CMD [ "node", "app.js" ]
