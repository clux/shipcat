{{/* vim: set filetype=mustache: */}}


{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "chart.chart" -}}
{{- printf "%s-%s" .Values.name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" -}}
{{- end -}}

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
