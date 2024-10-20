MYSQL_STRING := "mysql://root:password@10.97.212.193:13306/process_dispatcher"
MIGRATION_PATH := "./db/migrations"
NEW_MIGRATION_NAME := "new_migration"

MVP_MYSQL_STRING := "mysql://root:password@10.97.212.193:13306/mvp"
MVP_MIGRATION_PATH := "./db/mvp_migrations"
MVP_NEW_MIGRATION_NAME := "sources"

migrate.make:
	sqlx migrate add $(NEW_MIGRATION_NAME) -r -t --source $(MIGRATION_PATH)
migrate:
	sqlx migrate run --source $(MIGRATION_PATH) --database-url $(MYSQL_STRING)
migrate.dry-run:
	sqlx migrate run --source $(MIGRATION_PATH) --database-url $(MYSQL_STRING) --dry-run
migrate.revert:
	sqlx migrate revert --source $(MIGRATION_PATH) --database-url $(MYSQL_STRING)
migrate.revert.dry-run:
	sqlx migrate revert --source $(MIGRATION_PATH) --database-url $(MYSQL_STRING) --dry-run
migrate.info:
	sqlx migrate info --source $(MIGRATION_PATH) --database-url $(MYSQL_STRING)

mvp.migrate.make:
	sqlx migrate add $(MVP_NEW_MIGRATION_NAME) -r -t --source $(MVP_MIGRATION_PATH)
mvp.migrate:
	sqlx migrate run --source $(MVP_MIGRATION_PATH) --database-url $(MVP_MYSQL_STRING)
mvp.migrate.dry-run:
	sqlx migrate run --source $(MVP_MIGRATION_PATH) --database-url $(MVP_MYSQL_STRING) --dry-run
mvp.migrate.revert:
	sqlx migrate revert --source $(MVP_MIGRATION_PATH) --database-url $(MVP_MYSQL_STRING)
mvp.migrate.revert.dry-run:
	sqlx migrate revert --source $(MVP_MIGRATION_PATH) --database-url $(MVP_MYSQL_STRING) --dry-run
mvp.migrate.info:
	sqlx migrate info --source $(MVP_MIGRATION_PATH) --database-url $(MYSQL_STRING)

exec-mysql-client:
#	kubectl -n default exec mysql-client -it -c mysql-client -- mysql -h mysql-service -P 13306 -ppassword
	mysql -h 10.97.212.193 -P 13306 -u root -ppassword

fmt:
	cargo fmt

