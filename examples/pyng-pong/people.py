from typing import TypeVar, Generic, Union, Optional, Protocol, Tuple, List, Any, Self
from types import TracebackType

from rio_rs_guest.rio_service import exports
from rio_rs_guest.rio_service.types import Err



class Service(exports.people.Service):
    _id: str
    def __init__(self, id):
        self._id = id

    def id(self):
        return self._id
