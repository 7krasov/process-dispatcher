K8S_DIR := ops/k8s/minikube

run-local-app:
	clear && HTTP_PORT=8888 HOST_NAME=host1 cargo run --bin process_supervisor
exec:
	docker compose -f ../../docker-compose.yml exec process_supervisor_coding bash


deploy-minikube-project:
	kubectl create namespace process-supervisor || true
#	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-controller-role.yml
#	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-controller-serviceaccount.yml
#	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-controller-rolebinding.yml
#	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-controller-deployment.yml
	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-role.yml
	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-serviceaccount.yml
	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-rolebinding.yml
	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-deployment.yml
	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-service.yml
	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-podmonitor.yml
	kubectl apply -n process-supervisor -f $(K8S_DIR)/ps-scaledobject.yml

delete-minikube-project:
	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-scaledobject.yml || true
	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-podmonitor.yml || true
	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-service.yml || true
	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-deployment.yml || true
	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-rolebinding.yml || true
	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-serviceaccount.yml || true
	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-role.yml || true
#	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-controller-deployment.yml || true
#	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-controller-rolebinding.yml || true
#	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-controller-serviceaccount.yml || true
#	kubectl delete -n process-supervisor -f $(K8S_DIR)/ps-controller-role.yml || true
	kubectl delete namespace process-supervisor || true

restart-minikube-supervisor-deployment:
	kubectl -n process-supervisor rollout restart deployment process-supervisor-app-deployment

#restart-minikube-supervisor-controller-deployment:
#	kubectl -n process-supervisor rollout restart deployment processing-supervisor-controller-deployment

#terminate-supervisors:
#	#this way controller will know it should patch all pods so they see terminate-supervisors=true and terminate itself
#	kubectl annotate deployment processing-supervisor-controller-deployment -n process-supervisor terminate-supervisors=true

remove-pod-finalizer:
	#delete pod command will not work even with --force. We should remove finalizer from this pod first
	kubectl patch pod process-supervisor-app-deployment-c54465b49-5d29k -n process-supervisor -p '{"metadata":{"finalizers":[]}}' --type=merge

list-pods-minikube-project:
	kubectl -n process-supervisor get pods

create-docker-secret:
	echo "Please enter your password: " && stty -echo && read DOCKER_PASSWORD && stty echo && echo $${DOCKER_PASSWORD}; \
	kubectl -n process-supervisor create secret docker-registry 7krasov-docker --docker-username=7krasov --docker-password=$${DOCKER_PASSWORD} --docker-email=7krasov@gmail.com
build-k8s-image:
	#cargo build --target x86_64-unknown-linux-gnu --release
	#cargo build --target aarch64-unknown-linux-gnu --release
	#docker build --platform linux/amd64 -t 7krasov/process-supervisor.k8s.base:latest -f ./ops/docker/k8s.supervisor.base.Dockerfile .
	docker build --platform linux/arm64/v8 -t 7krasov/process-supervisor.k8s.base:latest -f ./ops/docker/k8s.supervisor.base.Dockerfile .
push-k8s-image:
	#docker push --platform linux/amd64 7krasov/process-supervisor.k8s.base:latest
	docker push --platform linux/arm64/v8 7krasov/process-supervisor.k8s.base:latest

release-k8s-image-mac:
	docker build --platform linux/arm64/v8 -t 7krasov/process-supervisor.k8s.base:latest -f ./ops/docker/k8s.supervisor.base.Dockerfile .
	docker push --platform linux/arm64/v8 7krasov/process-supervisor.k8s.base:latest

release-k8s-image-linux:
	docker build --platform linux/amd64 -t 7krasov/process-supervisor.k8s.base:latest -f ./ops/docker/k8s.supervisor.base.Dockerfile .
	docker push --platform linux/amd64 7krasov/process-supervisor.k8s.base:latest


expose-deployment-port:
	kubectl expose deployment processes-supervisor-app-deployment --type=LoadBalancer --port=8080
#	kubectl expose deployment processes-supervisor-app-deployment --type=NodePort --port=8080
run-minikube-tunnel:
	minikube tunnel

