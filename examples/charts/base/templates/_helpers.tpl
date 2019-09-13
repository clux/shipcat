{{/* vim: set filetype=mustache: */}}


{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "chart.shipcatRefs" }}
    app.kubernetes.io/name: {{ .Values.name }}
    app.kubernetes.io/version: {{ .Values.version }}
    app.kubernetes.io/managed-by: shipcat
  ownerReferences:
  - apiVersion: babylontech.co.uk/v1
    kind: ShipcatManifest
    controller: false
    name: {{ .Values.name }}
    uid: {{ .Values.uid }}
{{- end }}


{{- define "container-env" -}}
{{- range $k, $v := .plain }}
- name: {{ $k }}
  value: {{ $v | quote }}
{{- end }}
{{- $service := $.root.Values.name }}
{{- range $i, $name := .secrets }}
- name: {{ $name }}
  valueFrom:
    secretKeyRef:
      name: {{ $service }}-secrets
      key: {{ $name }}
{{- end }}
{{- end -}}
