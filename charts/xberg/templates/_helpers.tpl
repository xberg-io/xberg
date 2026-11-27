{{/*
Expand the name of the chart.
*/}}
{{- define "xberg.name" -}}
{{- default .Chart.Name .Values.nameOverride | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Create a default fully qualified app name.
*/}}
{{- define "xberg.fullname" -}}
{{- if .Values.fullnameOverride }}
{{- .Values.fullnameOverride | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- $name := default .Chart.Name .Values.nameOverride }}
{{- if contains $name .Release.Name }}
{{- .Release.Name | trunc 63 | trimSuffix "-" }}
{{- else }}
{{- printf "%s-%s" .Release.Name $name | trunc 63 | trimSuffix "-" }}
{{- end }}
{{- end }}
{{- end }}

{{/*
Create chart name and version as used by the chart label.
*/}}
{{- define "xberg.chart" -}}
{{- printf "%s-%s" .Chart.Name .Chart.Version | replace "+" "_" | trunc 63 | trimSuffix "-" }}
{{- end }}

{{/*
Common labels.
*/}}
{{- define "xberg.labels" -}}
helm.sh/chart: {{ include "xberg.chart" . }}
{{ include "xberg.selectorLabels" . }}
{{- if .Chart.AppVersion }}
app.kubernetes.io/version: {{ .Chart.AppVersion | quote }}
{{- end }}
app.kubernetes.io/managed-by: {{ .Release.Service }}
{{- end }}

{{/*
Selector labels.
*/}}
{{- define "xberg.selectorLabels" -}}
app.kubernetes.io/name: {{ include "xberg.name" . }}
app.kubernetes.io/instance: {{ .Release.Name }}
{{- end }}

{{/*
ServiceAccount name.
*/}}
{{- define "xberg.serviceAccountName" -}}
{{- if .Values.serviceAccount.create }}
{{- default (include "xberg.fullname" .) .Values.serviceAccount.name }}
{{- else }}
{{- default "default" .Values.serviceAccount.name }}
{{- end }}
{{- end }}

{{/*
Container image tag. ocr.tesseractModelSet=best with ocr.tesseractModelSource=image selects the
tessdata_best twin of the chosen tag: "" -> <appVersion>-best, latest -> latest-best,
core -> core-best, <version>-core -> <version>-core-best. A tag already ending in -best is kept.
*/}}
{{- define "xberg.imageTag" -}}
{{- $tag := .Values.image.tag | default .Chart.AppVersion }}
{{- if and (eq .Values.ocr.tesseractModelSet "best") (eq .Values.ocr.tesseractModelSource "image") (not (hasSuffix "-best" $tag)) }}
{{- $tag = printf "%s-best" $tag }}
{{- end }}
{{- $tag }}
{{- end }}

{{/*
Container image reference.
*/}}
{{- define "xberg.image" -}}
{{- printf "%s/%s:%s" .Values.image.registry .Values.image.repository (include "xberg.imageTag" .) }}
{{- end }}

{{/*
Reject values that cannot work together.
*/}}
{{- define "xberg.validate" -}}
{{- if and (eq .Values.ocr.tesseractModelSet "best") .Values.xberg.tessdataPrefix }}
{{- fail "xberg.tessdataPrefix must be empty with ocr.tesseractModelSet=best: TESSDATA_PREFIX is used as given for every model set, so it would replace the best models" }}
{{- end }}
{{- end }}
