{{ define "redis-sidecar-resources" }}
requests:
  cpu: "50m"
  memory: "10Mi"
limits:
  cpu: "500m"
  memory: "200Mi"
{{- end }}

{{- define "redis-sidecar" }}
- name: redis-sidecar
  image: redis:4
  imagePullPolicy: IfNotPresent
  resources:
{{- if .resources }}
{{ toYaml .resources | indent 4 }}
{{- else }}
{{ include "redis-sidecar-resources" . | indent 4 }}
{{- end }}
  ports:
  - name: redis
    containerPort: 6379
    protocol: TCP
  livenessProbe:
    tcpSocket:
      port: redis
    initialDelaySeconds: 15
    periodSeconds: 15
{{- end }}
