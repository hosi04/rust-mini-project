from airflow import DAG
from airflow.providers.standard.operators.bash import BashOperator
from datetime import datetime, timedelta

default_args = {
    "owner": "hosi",
    "retries": 2,
    "retry_delay": timedelta(minutes=5),
}


with DAG(
    dag_id = "pull_products_data",
    default_args=default_args,
    schedule="0 * * * *",
    start_date=datetime(2024, 1, 1),
    catchup = False,
    tags = ["rust", "api", "products"],
) as dag:
    fetch_and_load = BashOperator(
        task_id="fetch_and_load_products",
        bash_command="/opt/airflow/project/target/debug/rust-api 2>&1",
    )