FROM nginx:1.21

RUN mkdir -p /var/www/html
COPY src/index.html src/utils.js src/websocket_test.js /var/www/html/
COPY src/views /var/www/html/views
COPY nginx.conf /etc/nginx/nginx.conf
