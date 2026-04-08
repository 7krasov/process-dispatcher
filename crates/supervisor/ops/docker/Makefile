build-image:
	cd ../../
	docker build --platform linux/amd64 -t 7krasov/processes-supervisor-app:latest -f ./supervisor.Dockerfile ../../
push-image:
	docker push 7krasov/processes-supervisor-app:latest
