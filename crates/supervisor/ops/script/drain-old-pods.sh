#!/bin/bash

NAMESPACE="process-supervisor"
DEPLOYMENT="process-supervisor-app-deployment"
ANNOTATION_KEY="drain"
ANNOTATION_VALUE="true"

kubectl get pods -n $NAMESPACE -l app=your-app-label -o name | while read pod; do
  kubectl annotate $pod -n $NAMESPACE drain=true --overwrite
done

