from datetime import timedelta


class AnonymityConf:
    def __init__(self, min_k: int, min_k_percental: float,
                 min_area_size: float): ...


class Point:
    def __init__(self, lat, lon): ...


class Trajectory:
    def __init__(self): ...
    def add_point(self, p: Point, time): ...


class Car:
    def __init__(self, trajectory, color="#0033ee",
                 record_delay=timedelta(seconds=60),
                 send_delay=timedelta(seconds=120),
                 drive_delay=timedelta(seconds=0),
                 annon_conf=AnonymityConf(10, 100.0, 50.0)): ...


class SimConfig:
    def __init__(self, bbox_p1, bbox_p2, map, cars, predict, predict_n,
                 server_url="localhost:8080", projection_from="EPSG:4326",
                 projection_to="EPSG:4326", step_delta=timedelta(seconds=60)
                 ): ...


def sim_start(config: SimConfig): ...
