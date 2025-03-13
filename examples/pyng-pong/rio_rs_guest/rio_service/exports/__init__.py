from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType
from enum import Flag, Enum, auto
from dataclasses import dataclass
from abc import abstractmethod
import weakref

from ..types import Result, Ok, Err, Some
from ..exports import people
from ..imports import messages

class People(Protocol):
    pass

class PeoplePing(Protocol):

    @abstractmethod
    def handle(self, target: people.Service, message: messages.Ping) -> messages.Pong:
        """
        Raises: `rio_service.types.Err(rio_service.imports.messages.PingPongError)`
        """
        raise NotImplementedError


class PeoplePong(Protocol):

    @abstractmethod
    def handle(self, target: people.Service, message: messages.Pong) -> messages.Ping:
        """
        Raises: `rio_service.types.Err(rio_service.imports.messages.PingPongError)`
        """
        raise NotImplementedError


