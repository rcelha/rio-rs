from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType

from rio_rs_guest.rio_service import exports
from rio_rs_guest.rio_service.types import Err


class PeoplePing(exports.PeoplePing):

    def handle(self, target: exports.people.Service, message: exports.messages.Ping) -> exports.messages.Pong:
        return exports.messages.Pong(count=message.count)


class PeoplePong(exports.PeoplePong):

    def handle(self, target: exports.people.Service, message: exports.messages.Pong) -> exports.messages.Ping:
        return exports.messages.Ping(count=message.count)
