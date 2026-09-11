{{/*
Common labels applied to every resource this chart creates.
*/}}
{{- define "entropa.labels" -}}
app.kubernetes.io/part-of: entropa
app.kubernetes.io/managed-by: {{ .Release.Service }}
helm.sh/chart: {{ .Chart.Name }}-{{ .Chart.Version | replace "+" "_" }}
{{- end }}

{{/*
Selector labels for a specific service name (must be stable across releases,
unlike the full label set above, since these are used to match Pods).
*/}}
{{- define "entropa.selectorLabels" -}}
app.kubernetes.io/name: {{ .serviceName }}
app.kubernetes.io/part-of: entropa
{{- end }}
