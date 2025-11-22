FROM node:20-alpine

WORKDIR /app

# Install dependencies first for better caching
COPY package*.json ./
COPY tsconfig.json vite.config.ts ./
COPY index.html ./

# Copy source
COPY src ./src

RUN npm install

# Dev server
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0", "--port", "5173"]
